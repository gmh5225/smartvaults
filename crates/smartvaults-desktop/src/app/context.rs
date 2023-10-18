// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::fmt;

use smartvaults_sdk::core::bitcoin::Txid;
use smartvaults_sdk::core::policy::Policy;
use smartvaults_sdk::core::signer::Signer;
use smartvaults_sdk::nostr::{EventId, Url};
use smartvaults_sdk::types::GetPolicy;
use smartvaults_sdk::{util, SmartVaults};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage {
    Dashboard,
    Vaults,
    AddVault,
    VaultBuilder,
    RestoreVault,
    Vault(EventId),
    PolicyTree(EventId),
    Spend(Option<GetPolicy>),
    Receive(Option<GetPolicy>),
    SelfTransfer,
    NewProof(Option<GetPolicy>),
    Activity,
    Proposal(EventId),
    Transaction { policy_id: EventId, txid: Txid },
    History,
    CompletedProposal(EventId),
    Addresses(Option<(EventId, Policy)>),
    Signers,
    RevokeAllSigners,
    Signer(EventId, Signer),
    AddSigner,
    //AddHWSigner,
    AddAirGapSigner,
    ShareSigner(EventId),
    Contacts,
    AddContact,
    Profile,
    EditProfile,
    Settings,
    Config,
    Relays,
    Relay(Url),
    AddRelay,
    ChangePassword,
    RecoveryKeys,
    WipeKeys,
    NostrConnect,
    AddNostrConnectSession,
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dashboard => write!(f, "Dashboard"),
            Self::Vaults => write!(f, "Vaults"),
            Self::AddVault => write!(f, "Add vault"),
            Self::VaultBuilder => write!(f, "Builder"),
            Self::RestoreVault => write!(f, "Restore vault"),
            Self::PolicyTree(_) => write!(f, "Tree"),
            Self::Vault(id) => write!(f, "Vault #{}", util::cut_event_id(*id)),
            Self::Spend(_) => write!(f, "Spend"),
            Self::Receive(_) => write!(f, "Receive"),
            Self::SelfTransfer => write!(f, "Self transfer"),
            Self::NewProof(_) => write!(f, "New Proof"),
            Self::Activity => write!(f, "Activity"),
            Self::Proposal(id) => write!(f, "Proposal #{}", util::cut_event_id(*id)),
            Self::Transaction { txid, .. } => write!(f, "Tx #{}", util::cut_txid(*txid)),
            Self::History => write!(f, "History"),
            Self::CompletedProposal(..) => write!(f, "Completed proposal"),
            Self::Addresses(..) => write!(f, "Addresses"),
            Self::Signers => write!(f, "Signers"),
            Self::RevokeAllSigners => write!(f, "Revoke all"),
            Self::Signer(id, ..) => write!(f, "Signer #{}", util::cut_event_id(*id)),
            Self::AddSigner => write!(f, "Add signer"),
            //Self::AddHWSigner => write!(f, "Add HW signer"),
            Self::AddAirGapSigner => write!(f, "Add AirGap signer"),
            Self::ShareSigner(id) => write!(f, "Share signer #{}", util::cut_event_id(*id)),
            Self::Contacts => write!(f, "Contacts"),
            Self::AddContact => write!(f, "Add"),
            Self::Profile => write!(f, "Profile"),
            Self::EditProfile => write!(f, "Edit profile"),
            Self::Settings => write!(f, "Settings"),
            Self::Config => write!(f, "Config"),
            Self::Relays => write!(f, "Relays"),
            Self::Relay(..) => write!(f, "Relay"),
            Self::AddRelay => write!(f, "Add relay"),
            Self::ChangePassword => write!(f, "Change password"),
            Self::RecoveryKeys => write!(f, "Recovery Keys"),
            Self::WipeKeys => write!(f, "Wipe Keys"),
            Self::NostrConnect => write!(f, "Connect"),
            Self::AddNostrConnectSession => write!(f, "Add session"),
        }
    }
}

impl Default for Stage {
    fn default() -> Self {
        Self::Dashboard
    }
}

impl Stage {
    pub fn is_breadcrumb_first_level(&self) -> bool {
        matches!(
            self,
            Stage::Dashboard
                | Stage::Vaults
                | Stage::Activity
                | Stage::History
                | Stage::Signers
                | Stage::Contacts
                | Stage::Settings
                | Stage::Profile
                | Stage::NostrConnect
        )
    }
}

pub struct Context {
    pub stage: Stage,
    pub client: SmartVaults,
    pub hide_balances: bool,
    pub breadcrumb: Vec<Stage>,
}

impl Context {
    pub fn new(stage: Stage, client: SmartVaults) -> Self {
        Self {
            stage: stage.clone(),
            client,
            hide_balances: false,
            breadcrumb: vec![stage],
        }
    }

    pub fn set_stage(&mut self, stage: Stage) {
        if self.breadcrumb.contains(&stage) {
            if let Some(index) = self.breadcrumb.iter().position(|s| s == &stage) {
                self.breadcrumb = self.breadcrumb.clone().into_iter().take(index).collect();
            }
        }
        self.breadcrumb.push(stage.clone());
        self.stage = stage;
    }

    pub fn toggle_hide_balances(&mut self) {
        self.hide_balances = !self.hide_balances;
    }

    pub fn reset_breadcrumb(&mut self) {
        self.breadcrumb.clear();
    }
}
