// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::secp256k1::XOnlyPublicKey;
use coinstr_core::{Policy, Proposal};
use nostr::nips::nip04;
use nostr::{Event, EventBuilder, EventId, Keys, Tag};
use thiserror::Error;

use super::constants::{POLICY_KIND, PROPOSAL_KIND, SHARED_KEY_KIND};
use super::util::{Encryption, EncryptionError};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Keys(#[from] nostr::key::Error),
    #[error(transparent)]
    EventBuilder(#[from] nostr::event::builder::Error),
    #[error(transparent)]
    NIP04(#[from] nostr::nips::nip04::Error),
    #[error(transparent)]
    Encryption(#[from] EncryptionError),
}

pub trait CoinstrEventBuilder {
    fn shared_key(
        keys: &Keys,
        shared_key: &Keys,
        receiver: &XOnlyPublicKey,
        policy_id: EventId,
    ) -> Result<Event, Error> {
        let encrypted_shared_key = nip04::encrypt(
            &keys.secret_key()?,
            receiver,
            shared_key.secret_key()?.display_secret().to_string(),
        )?;
        let event: Event = EventBuilder::new(
            SHARED_KEY_KIND,
            encrypted_shared_key,
            &[
                Tag::Event(policy_id, None, None),
                Tag::PubKey(*receiver, None),
            ],
        )
        .to_event(keys)?;
        Ok(event)
    }

    fn policy(
        shared_key: &Keys,
        policy: &Policy,
        nostr_pubkeys: &[XOnlyPublicKey],
    ) -> Result<Event, Error> {
        let content: String = policy.encrypt_with_keys(shared_key)?;
        let tags: Vec<Tag> = nostr_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        Ok(EventBuilder::new(POLICY_KIND, content, &tags).to_event(shared_key)?)
    }

    fn proposal(
        shared_key: &Keys,
        policy_id: EventId,
        proposal: &Proposal,
        nostr_pubkeys: &[XOnlyPublicKey],
    ) -> Result<Event, Error> {
        let mut tags: Vec<Tag> = nostr_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        tags.push(Tag::Event(policy_id, None, None));
        let content: String = proposal.encrypt_with_keys(shared_key)?;
        Ok(EventBuilder::new(PROPOSAL_KIND, content, &tags).to_event(shared_key)?)
    }
}

impl CoinstrEventBuilder for EventBuilder {}