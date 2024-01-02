// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Error

use std::array::TryFromSliceError;

use nostr::nips::nip44;
use smartvaults_core::bitcoin::hashes;
use smartvaults_core::bitcoin::psbt::PsbtParseError;
use smartvaults_core::bitcoin::{address, consensus};
use smartvaults_core::miniscript::descriptor::DescriptorKeyParseError;
use smartvaults_core::signer::Error as CoreSignerError;
use smartvaults_core::util::hex;
use smartvaults_core::{miniscript, policy, proposal, secp256k1};
use thiserror::Error;

use super::core::SchemaError;
use super::network;

/// Protocol V2 Error
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    BitcoinConsensus(#[from] consensus::encode::Error),
    #[error(transparent)]
    Secp256k1(#[from] secp256k1::Error),
    #[error(transparent)]
    Hash(#[from] hashes::Error),
    #[error(transparent)]
    BitcoinHex(#[from] hashes::hex::Error),
    #[error(transparent)]
    Hex(#[from] hex::Error),
    #[error(transparent)]
    Policy(#[from] policy::Error),
    #[error(transparent)]
    Proposal(#[from] proposal::Error),
    #[error(transparent)]
    Network(#[from] network::Error),
    #[error(transparent)]
    Address(#[from] address::Error),
    #[error(transparent)]
    Psbt(#[from] PsbtParseError),
    #[error(transparent)]
    Miniscript(#[from] miniscript::Error),
    #[error(transparent)]
    NIP44(#[from] nip44::Error),
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error(transparent)]
    CoreSigner(#[from] CoreSignerError),
    #[error(transparent)]
    Proto(#[from] prost::DecodeError),
    #[error(transparent)]
    Keys(#[from] nostr::key::Error),
    #[error(transparent)]
    EventBuilder(#[from] nostr::event::builder::Error),
    #[error(transparent)]
    DescriptorKeyParse(#[from] DescriptorKeyParseError),
    #[error(transparent)]
    TryFromSlice(#[from] TryFromSliceError),
    #[error("{0} not found")]
    NotFound(String),
    #[error("proposal already finalized")]
    ProposalAlreadyFinalized,
}
