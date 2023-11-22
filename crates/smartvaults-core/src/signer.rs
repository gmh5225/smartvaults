// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::BTreeMap;

use keechain_core::bips::bip32::{self, Bip32, ChildNumber, DerivationPath, Fingerprint};
use keechain_core::bips::bip48::ScriptType;
use keechain_core::bitcoin::Network;
use keechain_core::descriptors::{self, ToDescriptor};
use keechain_core::miniscript::DescriptorPublicKey;
use keechain_core::{ColdcardGenericJson, Purpose, Seed};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::SECP256K1;

const PURPOSES: [Purpose; 3] = [
    Purpose::BIP86,
    Purpose::BIP48 {
        script: ScriptType::P2WSH,
    },
    Purpose::BIP48 {
        script: ScriptType::P2TR,
    },
];

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    BIP32(#[from] bip32::Error),
    #[error(transparent)]
    Descriptor(#[from] descriptors::Error),
    #[error(transparent)]
    Coldcard(#[from] keechain_core::export::coldcard::Error),
    #[error("fingerprint not match")]
    FingerprintNotMatch,
    #[error("network not match")]
    NetworkNotMatch,
    #[error("derivation path not found")]
    DerivationPathNotFound,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CoreSigner {
    fingerprint: Fingerprint,
    descriptors: BTreeMap<Purpose, DescriptorPublicKey>,
    // TODO: keep type?
}

impl CoreSigner {
    pub fn new(
        fingerprint: Fingerprint,
        descriptors: BTreeMap<Purpose, DescriptorPublicKey>,
        network: Network,
    ) -> Result<Self, Error> {
        // Check descriptors
        for descriptor in descriptors.values() {
            // Check if fingerprint match
            if fingerprint != descriptor.master_fingerprint() {
                return Err(Error::FingerprintNotMatch);
            }

            // Check network
            let path: DerivationPath = descriptor
                .full_derivation_path()
                .ok_or(Error::DerivationPathNotFound)?;
            let mut path_iter = path.into_iter();
            let _purpose = path_iter.next();
            let res: bool = match path_iter.next() {
                Some(ChildNumber::Hardened { index }) => match network {
                    Network::Bitcoin => *index == 0, // Mainnet
                    _ => *index == 1,                // Testnet, Signer or Regtest
                },
                _ => false,
            };

            if !res {
                return Err(Error::NetworkNotMatch);
            }
        }

        // Compose signer
        Ok(Self {
            fingerprint,
            descriptors,
        })
    }

    /// Compose [`Signer`] from [`Seed`]
    pub fn from_seed(seed: Seed, account: Option<u32>, network: Network) -> Result<Self, Error> {
        let mut descriptors: BTreeMap<Purpose, DescriptorPublicKey> = BTreeMap::new();

        // Derive descriptors
        for purpose in PURPOSES.into_iter() {
            let descriptor = seed.to_descriptor(purpose, account, false, network, &SECP256K1)?;
            descriptors.insert(purpose, descriptor);
        }

        Self::new(seed.fingerprint(network, &SECP256K1)?, descriptors, network)
    }

    /// Compose [`Signer`] from Coldcard generic JSON (`coldcard-export.json`)
    pub fn from_coldcard(coldcard: ColdcardGenericJson, network: Network) -> Result<Self, Error> {
        let mut descriptors: BTreeMap<Purpose, DescriptorPublicKey> = BTreeMap::new();

        // Derive descriptors
        for purpose in PURPOSES.into_iter() {
            let descriptor = coldcard.descriptor(purpose)?;
            descriptors.insert(purpose, descriptor);
        }

        Self::new(coldcard.fingerprint(), descriptors, network)
    }

    pub fn fingerprint(&self) -> Fingerprint {
        self.fingerprint
    }

    pub fn descriptors(&self) -> &BTreeMap<Purpose, DescriptorPublicKey> {
        &self.descriptors
    }

    pub fn descriptor(&self, purpose: Purpose) -> Option<DescriptorPublicKey> {
        self.descriptors.get(&purpose).cloned()
    }
}
