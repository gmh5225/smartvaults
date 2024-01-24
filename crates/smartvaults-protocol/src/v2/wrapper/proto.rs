// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use core::str::FromStr;

use smartvaults_core::secp256k1::XOnlyPublicKey;

use super::Wrapper;
use crate::v2::proto::wrapper::{
    ProtoSharedSignerInvite, ProtoVaultInvite, ProtoWrapper, ProtoWrapperObject,
};
use crate::v2::{Error, SharedSigner, Vault};

impl From<&Wrapper> for ProtoWrapper {
    fn from(wrapper: &Wrapper) -> Self {
        ProtoWrapper {
            object: Some(match wrapper {
                Wrapper::VaultInvite { vault, sender } => {
                    ProtoWrapperObject::VaultInvite(ProtoVaultInvite {
                        vault: Some(vault.into()),
                        sender: sender.map(|p| p.to_string()),
                    })
                }
                Wrapper::SharedSignerInvite {
                    shared_signer,
                    sender,
                } => ProtoWrapperObject::SharedSignerInvite(ProtoSharedSignerInvite {
                    shared_signer: Some(shared_signer.into()),
                    sender: sender.map(|p| p.to_string()),
                }),
            }),
        }
    }
}

impl TryFrom<ProtoWrapper> for Wrapper {
    type Error = Error;

    fn try_from(wrapper: ProtoWrapper) -> Result<Self, Self::Error> {
        match wrapper.object {
            Some(obj) => match obj {
                ProtoWrapperObject::VaultInvite(v) => {
                    let vault = v.vault.ok_or(Error::NotFound(String::from("vault")))?;
                    Ok(Self::VaultInvite {
                        vault: Vault::try_from(vault)?,
                        sender: match v.sender {
                            Some(public_key) => Some(XOnlyPublicKey::from_str(&public_key)?),
                            None => None,
                        },
                    })
                }
                ProtoWrapperObject::SharedSignerInvite(s) => {
                    let shared_signer = s
                        .shared_signer
                        .ok_or(Error::NotFound(String::from("shared signer")))?;
                    Ok(Self::SharedSignerInvite {
                        shared_signer: SharedSigner::try_from(shared_signer)?,
                        sender: match s.sender {
                            Some(public_key) => Some(XOnlyPublicKey::from_str(&public_key)?),
                            None => None,
                        },
                    })
                }
            },
            None => Err(Error::NotFound(String::from("protobuf wrapper obj"))),
        }
    }
}