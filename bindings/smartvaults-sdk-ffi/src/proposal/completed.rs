// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::sync::Arc;

use nostr_sdk_ffi::EventId;
use smartvaults_sdk::core::proposal;
use smartvaults_sdk::types;

#[derive(Clone)]
pub enum CompletedProposal {
    Spending {
        txid: String,
        description: String,
    },
    ProofOfReserve {
        descriptor: String,
        message: String,
        psbt: String,
    },
}

impl From<proposal::CompletedProposal> for CompletedProposal {
    fn from(value: proposal::CompletedProposal) -> Self {
        match value {
            proposal::CompletedProposal::Spending { description, tx } => Self::Spending {
                txid: tx.txid().to_string(),
                description,
            },
            proposal::CompletedProposal::ProofOfReserve {
                descriptor,
                message,
                psbt,
            } => Self::ProofOfReserve {
                descriptor: descriptor.to_string(),
                message,
                psbt: psbt.to_string(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct GetCompletedProposal {
    inner: types::GetCompletedProposal,
}

impl From<types::GetCompletedProposal> for GetCompletedProposal {
    fn from(inner: types::GetCompletedProposal) -> Self {
        Self { inner }
    }
}

impl GetCompletedProposal {
    pub fn completed_proposal_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.completed_proposal_id.into())
    }

    pub fn policy_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.policy_id.into())
    }

    pub fn completed_proposal(&self) -> CompletedProposal {
        self.inner.proposal.clone().into()
    }
}