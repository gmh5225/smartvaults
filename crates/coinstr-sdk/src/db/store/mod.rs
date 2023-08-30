// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

//! Store

#![allow(clippy::type_complexity)]

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ops::Add;
use std::path::Path;
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use coinstr_core::bitcoin::{Network, Txid};
use coinstr_core::proposal::{CompletedProposal, Proposal};
use coinstr_core::ApprovedProposal;
use coinstr_protocol::v1::util::serde::Serde;
use coinstr_protocol::v1::util::Encryption;
use nostr_sdk::event::id::EventId;
use nostr_sdk::secp256k1::{SecretKey, XOnlyPublicKey};
use nostr_sdk::{Event, Keys, Timestamp};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::config::DbConfig;
use rusqlite::OpenFlags;
use tokio::sync::RwLock;

mod connect;
mod contacts;
mod label;
mod policy;
mod relays;
mod signers;
mod timechain;
mod utxos;

use super::migration::{self, STARTUP_SQL};
use super::model::{
    GetApproval, GetApprovedProposals, GetCompletedProposal, GetNotifications, GetProposal,
};
use super::Error;
use crate::constants::BLOCK_HEIGHT_SYNC_INTERVAL;
use crate::types::Notification;

pub(crate) type SqlitePool = r2d2::Pool<SqliteConnectionManager>;
pub(crate) type PooledConnection = r2d2::PooledConnection<SqliteConnectionManager>;

#[derive(Debug, Clone, Default)]
pub struct BlockHeight {
    height: Arc<AtomicU32>,
    last_sync: Arc<RwLock<Option<Timestamp>>>,
}

impl BlockHeight {
    pub fn block_height(&self) -> u32 {
        self.height.load(Ordering::SeqCst)
    }

    pub fn set_block_height(&self, block_height: u32) {
        let _ = self
            .height
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |_| Some(block_height));
    }

    pub async fn is_synced(&self) -> bool {
        let last_sync = self.last_sync.read().await;
        let last_sync: Timestamp = last_sync.unwrap_or_else(|| Timestamp::from(0));
        last_sync.add(BLOCK_HEIGHT_SYNC_INTERVAL) > Timestamp::now()
    }

    pub async fn just_synced(&self) {
        let mut last_sync = self.last_sync.write().await;
        *last_sync = Some(Timestamp::now());
    }
}

/// Store
#[derive(Debug, Clone)]
pub struct Store {
    pool: SqlitePool,
    keys: Keys,
    network: Network,
    pub(crate) block_height: BlockHeight,
    nostr_connect_auto_approve: Arc<RwLock<HashMap<XOnlyPublicKey, Timestamp>>>,
}

impl Drop for Store {
    fn drop(&mut self) {}
}

impl Store {
    /// Open new database
    pub fn open<P>(user_db_path: P, keys: &Keys, network: Network) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let manager = SqliteConnectionManager::file(user_db_path.as_ref())
            .with_flags(OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)
            .with_init(|c| c.execute_batch(STARTUP_SQL));
        let pool = r2d2::Pool::new(manager)?;
        migration::run(&mut pool.get()?)?;
        Ok(Self {
            pool,
            keys: keys.clone(),
            network,
            nostr_connect_auto_approve: Arc::new(RwLock::new(HashMap::new())),
            block_height: BlockHeight::default(),
        })
    }

    /// Close db
    pub fn close(self) {
        drop(self);
    }

    pub fn wipe(&self) -> Result<(), Error> {
        let mut conn = self.pool.get()?;

        // Reset DB
        conn.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, true)?;
        conn.execute("VACUUM;", [])?;
        conn.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, false)?;

        // Execute migrations
        conn.execute_batch(STARTUP_SQL)?;
        migration::run(&mut conn)?;

        Ok(())
    }

    pub fn block_height(&self) -> u32 {
        self.block_height.block_height()
    }

    pub fn shared_key_exists_for_policy(&self, policy_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare("SELECT EXISTS(SELECT 1 FROM shared_keys WHERE policy_id = ? LIMIT 1);")?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn save_shared_key(&self, policy_id: EventId, shared_key: Keys) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR IGNORE INTO shared_keys (policy_id, shared_key) VALUES (?, ?);",
            (
                policy_id.to_hex(),
                shared_key.secret_key()?.encrypt_with_keys(&self.keys)?,
            ),
        )?;
        Ok(())
    }

    pub fn get_shared_key(&self, policy_id: EventId) -> Result<Keys, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("SELECT shared_key FROM shared_keys WHERE policy_id = ?;")?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let row = rows.next()?.ok_or(Error::NotFound("shared_key".into()))?;
        let sk: String = row.get(0)?;
        let sk = SecretKey::decrypt_with_keys(&self.keys, sk)?;
        Ok(Keys::new(sk))
    }

    pub fn get_nostr_pubkeys(&self, policy_id: EventId) -> Result<Vec<XOnlyPublicKey>, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("SELECT public_key FROM nostr_public_keys WHERE policy_id = ?;")?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let mut pubkeys = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let public_key: String = row.get(0)?;
            pubkeys.push(XOnlyPublicKey::from_str(&public_key)?);
        }
        Ok(pubkeys)
    }

    pub fn proposal_exists(&self, proposal_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM proposals WHERE proposal_id = ? LIMIT 1);",
        )?;
        let mut rows = stmt.query([proposal_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn save_proposal(
        &self,
        proposal_id: EventId,
        policy_id: EventId,
        proposal: Proposal,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR IGNORE INTO proposals (proposal_id, policy_id, proposal) VALUES (?, ?, ?);",
            (
                proposal_id.to_hex(),
                policy_id.to_hex(),
                proposal.encrypt_with_keys(&self.keys)?,
            ),
        )?;

        // Freeze UTXOs
        for txin in proposal.psbt().unsigned_tx.input.into_iter() {
            self.freeze_utxo(txin.previous_output, policy_id, Some(proposal_id))?;
        }

        tracing::info!("Spending proposal {proposal_id} saved");
        Ok(())
    }

    pub async fn get_proposals(&self) -> Result<Vec<GetProposal>, Error> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            let conn = this.pool.get()?;
            let mut stmt =
                conn.prepare_cached("SELECT proposal_id, policy_id, proposal FROM proposals;")?;
            let mut rows = stmt.query([])?;
            let mut proposals = Vec::new();
            while let Ok(Some(row)) = rows.next() {
                let proposal_id: String = row.get(0)?;
                let policy_id: String = row.get(1)?;
                let proposal: String = row.get(2)?;

                let proposal_id = EventId::from_hex(proposal_id)?;
                let policy_id = EventId::from_hex(policy_id)?;
                let proposal = Proposal::decrypt_with_keys(&this.keys, proposal)?;
                let approved_proposals =
                    this.get_approved_proposals_by_proposal_id(proposal_id, &conn)?;

                proposals.push(GetProposal {
                    proposal_id,
                    policy_id,
                    signed: proposal.finalize(approved_proposals, this.network).is_ok(),
                    proposal,
                });
            }
            Ok(proposals)
        })
        .await?
    }

    fn get_proposal_ids_by_policy_id(&self, policy_id: EventId) -> Result<Vec<EventId>, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("SELECT proposal_id FROM proposals WHERE policy_id = ?;")?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let mut ids = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let proposal_id: String = row.get(0)?;
            ids.push(EventId::from_hex(proposal_id)?);
        }
        Ok(ids)
    }

    pub fn get_proposals_by_policy_id(
        &self,
        policy_id: EventId,
    ) -> Result<Vec<GetProposal>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn
            .prepare_cached("SELECT proposal_id, proposal FROM proposals WHERE policy_id = ?;")?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let mut proposals = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let proposal_id: String = row.get(0)?;
            let proposal: String = row.get(1)?;

            let proposal_id = EventId::from_hex(proposal_id)?;
            let proposal = Proposal::decrypt_with_keys(&self.keys, proposal)?;
            let approved_proposals =
                self.get_approved_proposals_by_proposal_id(proposal_id, &conn)?;

            proposals.push(GetProposal {
                proposal_id,
                policy_id,
                signed: proposal.finalize(approved_proposals, self.network).is_ok(),
                proposal,
            });
        }
        Ok(proposals)
    }

    pub fn get_proposal(&self, proposal_id: EventId) -> Result<GetProposal, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT policy_id, proposal FROM proposals WHERE proposal_id = ? LIMIT 1;",
        )?;
        let mut rows = stmt.query([proposal_id.to_hex()])?;
        let row = rows.next()?.ok_or(Error::NotFound("proposal".into()))?;
        let policy_id: String = row.get(0)?;
        let proposal: String = row.get(1)?;

        let policy_id = EventId::from_hex(policy_id)?;
        let proposal = Proposal::decrypt_with_keys(&self.keys, proposal)?;
        let approved_proposals = self.get_approved_proposals_by_proposal_id(proposal_id, &conn)?;

        Ok(GetProposal {
            proposal_id,
            policy_id,
            signed: proposal.finalize(approved_proposals, self.network).is_ok(),
            proposal,
        })
    }

    pub fn delete_proposal(&self, proposal_id: EventId) -> Result<(), Error> {
        self.set_event_as_deleted(proposal_id)?;

        // Delete notification
        self.delete_notification(proposal_id)?;

        // Delete proposal
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM proposals WHERE proposal_id = ?;",
            [proposal_id.to_hex()],
        )?;

        // Delete approvals
        conn.execute(
            "DELETE FROM approved_proposals WHERE proposal_id = ?;",
            [proposal_id.to_hex()],
        )?;

        // Delete frozen UTXOs
        conn.execute(
            "DELETE FROM frozen_utxos WHERE proposal_id = ?;",
            [proposal_id.to_hex()],
        )?;

        tracing::info!("Deleted proposal {proposal_id}");
        Ok(())
    }

    pub fn get_approvals_by_proposal_id(
        &self,
        proposal_id: EventId,
    ) -> Result<Vec<GetApproval>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("SELECT approval_id, public_key, approved_proposal, timestamp FROM approved_proposals WHERE proposal_id = ?;")?;
        let mut rows = stmt.query([proposal_id.to_hex()])?;
        let mut approvals = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let approval_id: String = row.get(0)?;
            let public_key: String = row.get(1)?;
            let json: String = row.get(2)?;
            let timestamp: u64 = row.get(3)?;
            approvals.push(GetApproval {
                approval_id: EventId::from_hex(approval_id)?,
                public_key: XOnlyPublicKey::from_str(&public_key)?,
                approved_proposal: ApprovedProposal::decrypt_with_keys(&self.keys, json)?,
                timestamp: Timestamp::from(timestamp),
            });
        }
        Ok(approvals)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    fn get_approved_proposals_by_proposal_id(
        &self,
        proposal_id: EventId,
        conn: &PooledConnection,
    ) -> Result<Vec<ApprovedProposal>, Error> {
        let mut stmt = conn.prepare_cached(
            "SELECT approved_proposal FROM approved_proposals WHERE proposal_id = ?;",
        )?;
        let mut rows = stmt.query([proposal_id.to_hex()])?;
        let mut approvals = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let json: String = row.get(0)?;
            approvals.push(ApprovedProposal::decrypt_with_keys(&self.keys, json)?);
        }
        Ok(approvals)
    }

    pub fn get_approved_proposals_by_id(
        &self,
        proposal_id: EventId,
    ) -> Result<GetApprovedProposals, Error> {
        let GetProposal {
            policy_id,
            proposal,
            ..
        } = self.get_proposal(proposal_id)?;
        let approved_proposals = self.get_approvals_by_proposal_id(proposal_id)?;
        Ok(GetApprovedProposals {
            policy_id,
            proposal,
            approved_proposals: approved_proposals
                .iter()
                .map(
                    |GetApproval {
                         approved_proposal, ..
                     }| approved_proposal.clone(),
                )
                .collect(),
        })
    }

    pub fn save_approved_proposal(
        &self,
        proposal_id: EventId,
        author: XOnlyPublicKey,
        approval_id: EventId,
        approved_proposal: ApprovedProposal,
        timestamp: Timestamp,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR IGNORE INTO approved_proposals (approval_id, proposal_id, public_key, approved_proposal, timestamp) VALUES (?, ?, ?, ?, ?);",
            (approval_id.to_hex(), proposal_id.to_hex(), author.to_string(), approved_proposal.encrypt_with_keys(&self.keys)?, timestamp.as_u64()),
        )?;
        Ok(())
    }

    pub fn get_policy_id_by_approval_id(&self, approval_id: EventId) -> Result<EventId, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT proposal_id FROM approved_proposals WHERE approval_id = ? LIMIT 1;",
        )?;
        let mut rows = stmt.query([approval_id.to_hex()])?;
        let row = rows.next()?.ok_or(Error::NotFound("approval".into()))?;
        let proposal_id: String = row.get(0)?;
        let proposal_id = EventId::from_hex(proposal_id)?;
        let GetProposal { policy_id, .. } = self.get_proposal(proposal_id)?;
        Ok(policy_id)
    }

    pub fn approved_proposal_exists(&self, approved_proposal_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT EXISTS(SELECT 1 FROM approved_proposals WHERE approval_id = ? LIMIT 1);",
        )?;
        let mut rows = stmt.query([approved_proposal_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn delete_approval(&self, approval_id: EventId) -> Result<(), Error> {
        self.set_event_as_deleted(approval_id)?;

        // Delete notification
        self.delete_notification(approval_id)?;

        // Delete policy
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM approved_proposals WHERE approval_id = ?;",
            [approval_id.to_hex()],
        )?;
        tracing::info!("Deleted approval {approval_id}");
        Ok(())
    }

    pub fn completed_proposal_exists(&self, completed_proposal_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare("SELECT EXISTS(SELECT 1 FROM completed_proposals WHERE completed_proposal_id = ? LIMIT 1);")?;
        let mut rows = stmt.query([completed_proposal_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn save_completed_proposal(
        &self,
        completed_proposal_id: EventId,
        policy_id: EventId,
        completed_proposal: CompletedProposal,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR IGNORE INTO completed_proposals (completed_proposal_id, policy_id, completed_proposal) VALUES (?, ?, ?);",
            (completed_proposal_id.to_hex(), policy_id.to_hex(), completed_proposal.encrypt_with_keys(&self.keys)?),
        )?;
        tracing::info!("Completed proposal {completed_proposal_id} saved");
        Ok(())
    }

    pub fn completed_proposals(&self) -> Result<Vec<GetCompletedProposal>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT completed_proposal_id, policy_id, completed_proposal FROM completed_proposals;",
        )?;
        let mut rows = stmt.query([])?;
        let mut proposals = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let proposal_id: String = row.get(0)?;
            let policy_id: String = row.get(1)?;
            let proposal: String = row.get(2)?;
            proposals.push(GetCompletedProposal {
                policy_id: EventId::from_hex(policy_id)?,
                completed_proposal_id: EventId::from_hex(proposal_id)?,
                proposal: CompletedProposal::decrypt_with_keys(&self.keys, proposal)?,
            });
        }
        Ok(proposals)
    }

    pub fn completed_proposals_by_policy_id(
        &self,
        policy_id: EventId,
    ) -> Result<Vec<GetCompletedProposal>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT completed_proposal_id, completed_proposal FROM completed_proposals WHERE policy_id = ?;",
        )?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let mut proposals = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let proposal_id: String = row.get(0)?;
            let proposal: String = row.get(1)?;
            proposals.push(GetCompletedProposal {
                policy_id,
                completed_proposal_id: EventId::from_hex(proposal_id)?,
                proposal: CompletedProposal::decrypt_with_keys(&self.keys, proposal)?,
            });
        }
        Ok(proposals)
    }

    pub fn get_completed_proposal(
        &self,
        completed_proposal_id: EventId,
    ) -> Result<GetCompletedProposal, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT policy_id, completed_proposal FROM completed_proposals WHERE completed_proposal_id = ? LIMIT 1;")?;
        let mut rows = stmt.query([completed_proposal_id.to_hex()])?;
        let row = rows
            .next()?
            .ok_or(Error::NotFound("completed proposal".into()))?;
        let policy_id: String = row.get(0)?;
        let proposal: String = row.get(1)?;
        Ok(GetCompletedProposal {
            policy_id: EventId::from_hex(policy_id)?,
            completed_proposal_id,
            proposal: CompletedProposal::decrypt_with_keys(&self.keys, proposal)?,
        })
    }

    fn get_completed_proposal_ids_by_policy_id(
        &self,
        policy_id: EventId,
    ) -> Result<Vec<EventId>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT completed_proposal_id FROM completed_proposals WHERE policy_id = ?;",
        )?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let mut ids = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let completed_proposal_id: String = row.get(0)?;
            ids.push(EventId::from_hex(completed_proposal_id)?);
        }
        Ok(ids)
    }

    pub fn delete_completed_proposal(&self, completed_proposal_id: EventId) -> Result<(), Error> {
        self.set_event_as_deleted(completed_proposal_id)?;

        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM completed_proposals WHERE completed_proposal_id = ?;",
            [completed_proposal_id.to_hex()],
        )?;
        tracing::info!("Deleted completed proposal {completed_proposal_id}");
        Ok(())
    }

    pub(crate) fn get_description_by_txid(
        &self,
        policy_id: EventId,
        txid: Txid,
    ) -> Result<Option<String>, Error> {
        for GetCompletedProposal { proposal, .. } in self
            .completed_proposals_by_policy_id(policy_id)?
            .into_iter()
        {
            if let CompletedProposal::Spending {
                tx, description, ..
            } = proposal
            {
                if tx.txid() == txid {
                    return Ok(Some(description));
                }
            }
        }
        Ok(None)
    }

    pub(crate) fn get_txs_descriptions(
        &self,
        policy_id: EventId,
    ) -> Result<HashMap<Txid, String>, Error> {
        let mut map = HashMap::new();
        for GetCompletedProposal { proposal, .. } in self
            .completed_proposals_by_policy_id(policy_id)?
            .into_iter()
        {
            if let CompletedProposal::Spending {
                tx, description, ..
            } = proposal
            {
                if let Entry::Vacant(e) = map.entry(tx.txid()) {
                    e.insert(description);
                }
            }
        }
        Ok(map)
    }

    pub fn delete_generic_event_id(&self, event_id: EventId) -> Result<(), Error> {
        if self.policy_exists(event_id)? {
            self.delete_policy(event_id)?;
        } else if self.proposal_exists(event_id)? {
            self.delete_proposal(event_id)?;
        } else if self.approved_proposal_exists(event_id)? {
            self.delete_approval(event_id)?;
        } else if self.completed_proposal_exists(event_id)? {
            self.delete_completed_proposal(event_id)?;
        } else if self.signer_exists(event_id)? {
            self.delete_signer(event_id)?;
        } else if self.my_shared_signer_exists(event_id)? || self.shared_signer_exists(event_id)? {
            self.delete_shared_signer(event_id)?;
        } else {
            self.set_event_as_deleted(event_id)?;
        };

        Ok(())
    }

    pub fn save_event(&self, event: &Event) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("INSERT OR IGNORE INTO events (event_id, event) VALUES (?, ?);")?;
        stmt.execute((event.id.to_hex(), event.as_json()))?;
        Ok(())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_events(&self) -> Result<Vec<Event>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT event FROM events;")?;
        let mut rows = stmt.query([])?;
        let mut events: Vec<Event> = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let json: String = row.get(0)?;
            let event: Event = Event::from_json(json)?;
            events.push(event);
        }
        Ok(events)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_event_by_id(&self, event_id: EventId) -> Result<Event, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT event FROM events WHERE event_id = ? LIMIT 1;")?;
        let mut rows = stmt.query([event_id.to_hex()])?;
        let row = rows.next()?.ok_or(Error::NotFound("event".into()))?;
        let json: String = row.get(0)?;
        Ok(Event::from_json(json)?)
    }

    pub fn event_was_deleted(&self, event_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM events WHERE event_id = ? AND deleted = 1 LIMIT 1);",
        )?;
        let mut rows = stmt.query([event_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn set_event_as_deleted(&self, event_id: EventId) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("UPDATE events SET deleted = 1 WHERE event_id = ?")?;
        stmt.execute([event_id.to_hex()])?;
        Ok(())
    }

    pub fn save_pending_event(&self, event: &Event) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("INSERT OR IGNORE INTO pending_events (event) VALUES (?);")?;
        stmt.execute([event.as_json()])?;
        tracing::info!("Saved pending event {} (kind={:?})", event.id, event.kind);
        Ok(())
    }

    pub fn get_pending_events(&self) -> Result<Vec<Event>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT event FROM pending_events;")?;
        let mut rows = stmt.query([])?;
        let mut events: Vec<Event> = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let json: String = row.get(0)?;
            let event: Event = Event::from_json(json)?;
            events.push(event);
        }
        Ok(events)
    }

    pub fn save_notification(
        &self,
        event_id: EventId,
        notification: Notification,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "INSERT OR IGNORE INTO notifications (event_id, notification, timestamp) VALUES (?, ?, ?);",
        )?;
        stmt.execute((
            event_id.to_hex(),
            notification.as_json(),
            Timestamp::now().as_u64(),
        ))?;
        Ok(())
    }

    pub fn get_notifications(&self) -> Result<Vec<GetNotifications>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT notification, timestamp, seen FROM notifications ORDER BY timestamp DESC;",
        )?;
        let mut rows = stmt.query([])?;
        let mut notifications: Vec<GetNotifications> = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let json: String = row.get(0)?;
            let notification: Notification = Notification::from_json(json)?;
            let timestamp: u64 = row.get(1)?;
            let timestamp = Timestamp::from(timestamp);
            let seen: bool = row.get(2)?;
            notifications.push(GetNotifications {
                notification,
                timestamp,
                seen,
            });
        }
        Ok(notifications)
    }

    pub fn count_unseen_notifications(&self) -> Result<usize, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM notifications WHERE seen = 0;")?;
        let mut rows = stmt.query([])?;
        let row = rows
            .next()?
            .ok_or(Error::NotFound("count notifications".into()))?;
        let count: usize = row.get(0)?;
        Ok(count)
    }

    pub fn mark_notification_as_seen_by_id(&self, event_id: EventId) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("UPDATE notifications SET seen = 1 WHERE event_id = ?")?;
        stmt.execute([event_id.to_hex()])?;
        Ok(())
    }

    pub fn mark_notification_as_seen(&self, notification: Notification) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("UPDATE notifications SET seen = 1 WHERE notification = ?")?;
        stmt.execute([notification.as_json()])?;
        Ok(())
    }

    pub fn mark_all_notifications_as_seen(&self) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("UPDATE notifications SET seen = 1;")?;
        stmt.execute([])?;
        Ok(())
    }

    pub fn delete_all_notifications(&self) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("DELETE FROM notifications;")?;
        stmt.execute([])?;
        Ok(())
    }

    pub fn delete_notification(&self, event_id: EventId) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("DELETE FROM notifications WHERE event_id = ?")?;
        stmt.execute([event_id.to_hex()])?;
        Ok(())
    }
}
