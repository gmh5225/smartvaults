// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use clap::{Parser, Subcommand};
use coinstr_core::bitcoin::{Address, XOnlyPublicKey};
use coinstr_core::nostr_sdk::EventId;

pub mod io;
mod types;

use self::types::{CliNetwork, CliWordCount};
use crate::DEFAULT_RELAY;

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about)]
pub struct Cli {
    /// Network
    #[clap(short, long, value_enum, default_value_t = CliNetwork::Bitcoin)]
    pub network: CliNetwork,
    /// Relay
    #[clap(short, long, default_value_t = DEFAULT_RELAY.to_string())]
    pub relay: String,
    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Debug, Subcommand)]
pub enum CliCommand {
    /// Generate new keychain
    #[command(arg_required_else_help = true)]
    Generate {
        /// Keychain name
        #[arg(required = true)]
        name: String,
        /// Word count
        #[arg(value_enum, default_value_t = CliWordCount::W12)]
        word_count: CliWordCount,
        /// Password
        #[arg(long)]
        password: Option<String>,
        /// Passphrase
        #[arg(long)]
        passphrase: Option<String>,
    },
    /// Restore keychain
    #[command(arg_required_else_help = true)]
    Restore {
        /// Keychain name
        #[arg(required = true)]
        name: String,
    },
    /// Open keychain
    #[command(arg_required_else_help = true)]
    Open {
        /// Keychain name
        #[arg(required = true)]
        name: String,
    },
    /// List keychains
    List,
    /// Setting
    Setting {
        #[command(subcommand)]
        command: SettingCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum SettingCommand {
    /// Rename keychain
    #[command(arg_required_else_help = true)]
    Rename {
        /// Keychain name
        #[arg(required = true)]
        name: String,
        /// New keychain name
        #[arg(required = true)]
        new_name: String,
    },
    /// Change keychain password
    #[command(arg_required_else_help = true)]
    ChangePassword {
        /// Keychain name
        #[arg(required = true)]
        name: String,
    },
}

#[derive(Debug, Parser)]
#[command(name = "")]
pub enum Command {
    /// Inspect bitcoin and nostr keys
    Inspect,
    /// Save policy
    SavePolicy {
        /// Policy name
        #[arg(required = true)]
        name: String,
        /// Policy description
        #[arg(required = true)]
        description: String,
        /// Policy descriptor
        #[arg(required = true)]
        descriptor: String,
        /// Custom nostr pubkeys
        custom_pubkeys: Option<Vec<XOnlyPublicKey>>,
    },
    /// Create a spending proposal
    Spend {
        /// Policy id
        #[arg(required = true)]
        policy_id: EventId,
        /// To address
        #[arg(required = true)]
        to_address: Address,
        /// Amount in sat
        #[arg(required = true)]
        amount: u64,
        /// Description
        #[arg(required = true)]
        description: String,
        /// Taget blocks
        #[clap(short, long, default_value_t = 6)]
        target_blocks: usize,
    },
    /// Create a spending proposal (send all funds)
    SpendAll {
        /// Policy id
        #[arg(required = true)]
        policy_id: EventId,
        /// To address
        #[arg(required = true)]
        to_address: Address,
        /// Description
        #[arg(required = true)]
        description: String,
        /// Taget blocks
        #[clap(short, long, default_value_t = 6)]
        target_blocks: usize,
    },
    /// Approve a spending proposal
    Approve {
        /// Proposal id
        #[arg(required = true)]
        proposal_id: EventId,
    },
    /// Combine and broadcast the transaction
    Broadcast {
        /// Proposal id
        #[arg(required = true)]
        proposal_id: EventId,
    },
    /// Proof of Reserve commands
    #[command(arg_required_else_help = true)]
    Proof {
        #[command(subcommand)]
        command: ProofCommand,
    },
    /// Get data about policies and proposals
    #[command(arg_required_else_help = true)]
    Get {
        #[command(subcommand)]
        command: GetCommand,
    },
    /// Delete
    #[command(arg_required_else_help = true)]
    Delete {
        #[command(subcommand)]
        command: DeleteCommand,
    },
    /// Exit
    Exit,
}

#[derive(Debug, Subcommand)]
pub enum ProofCommand {
    /// New Proof Of Reserve
    New {
        /// Policy id
        #[arg(required = true)]
        policy_id: EventId,
        /// Message
        #[arg(required = true)]
        message: String,
    },
    /// Finalize Proof Of Reserve
    Finalize {
        /// Proposal id
        #[arg(required = true)]
        proposal_id: EventId,
    },
    /// Verify Proof Of Reserve
    Verify {
        /// Proposal id
        #[arg(required = true)]
        proposal_id: EventId,
    },
}

#[derive(Debug, Subcommand)]
pub enum GetCommand {
    /// Get contacts list from nostr
    Contacts,
    /// Get policies list from nostr
    Policies,
    /// Get policy by id
    Policy {
        /// Policy id
        #[arg(required = true)]
        policy_id: EventId,
        /// Export descriptor
        #[arg(long)]
        export: bool,
    },
    /// Get proposals list from nostr
    Proposals {
        /// Get completed proposals
        #[arg(long)]
        completed: bool,
    },
    /// Get proposal by id
    Proposal {
        /// Proposal id
        #[arg(required = true)]
        proposal_id: EventId,
    },
}

#[derive(Debug, Subcommand)]
pub enum DeleteCommand {
    /// Delete policy by id
    Policy {
        /// Policy id
        #[arg(required = true)]
        policy_id: EventId,
    },
    /// Delete proposal by id
    Proposal {
        /// Proposal id
        #[arg(required = true)]
        proposal_id: EventId,
        /// Is a completed proposals
        #[arg(long)]
        completed: bool,
    },
}