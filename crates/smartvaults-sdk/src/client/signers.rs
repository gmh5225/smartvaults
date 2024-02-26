// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::{BTreeSet, HashSet};

use nostr_sdk::prelude::*;
use smartvaults_core::miniscript::Descriptor;
use smartvaults_protocol::v2::constants::{SHARED_SIGNER_KIND_V2, SIGNER_KIND_V2};
use smartvaults_protocol::v2::{
    self, NostrPublicIdentifier, SharedSigner, SharedSignerInvite, Signer, SignerIdentifier,
};

use super::{Error, SmartVaults};
use crate::types::GetSharedSigner;

impl SmartVaults {
    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_signer_by_id(&self, signer_id: &SignerIdentifier) -> Result<Signer, Error> {
        self.storage.signer(signer_id).await
    }

    pub async fn delete_signer_by_id(&self, signer_id: &SignerIdentifier) -> Result<(), Error> {
        let public_key: PublicKey = self.nostr_public_key().await?;

        let signer: Signer = self.storage.signer(signer_id).await?;

        let nostr_public_identifier: NostrPublicIdentifier = signer.nostr_public_identifier();

        let filter: Filter = Filter::new()
            .kind(SIGNER_KIND_V2)
            .author(public_key)
            .identifier(nostr_public_identifier.to_string())
            .limit(1);
        let res: Vec<Event> = self
            .client
            .database()
            .query(vec![filter], Order::Desc)
            .await?;
        let signer_event: &Event = res.first().ok_or(Error::NotFound)?;

        let event = EventBuilder::new(Kind::EventDeletion, "", [Tag::event(signer_event.id)]);
        self.client.send_event_builder(event).await?;

        self.storage.delete_signer(signer_id).await;

        Ok(())
    }

    pub async fn save_signer(&self, signer: Signer) -> Result<SignerIdentifier, Error> {
        let nostr_signer = self.client.signer().await?;

        // Compose and publish event
        let event: Event = v2::signer::build_event(&nostr_signer, &signer).await?;
        self.client.send_event(event).await?;

        // Index signer
        let id: SignerIdentifier = signer.compute_id();
        self.storage.save_signer(id, signer).await;

        Ok(id)
    }

    pub async fn smartvaults_signer_exists(&self) -> bool {
        self.storage
            .signer_exists(&self.default_signer.compute_id())
            .await
    }

    pub async fn save_smartvaults_signer(&self) -> Result<SignerIdentifier, Error> {
        self.save_signer(self.default_signer.clone()).await
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn signers(&self) -> BTreeSet<Signer> {
        self.storage.signers().await.into_values().collect()
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn search_signer_by_descriptor(
        &self,
        descriptor: Descriptor<String>,
    ) -> Result<Signer, Error> {
        let descriptor: String = descriptor.to_string();
        for signer in self.storage.signers().await.into_values() {
            for desc in signer.descriptors().values() {
                let signer_descriptor: String = desc.to_string();
                if descriptor.contains(&signer_descriptor) {
                    return Ok(signer);
                }
            }
        }
        Err(Error::SignerNotFound)
    }

    /// Edit [Signer] metadata
    ///
    /// Args set to `None` aren't updated.
    pub async fn edit_signer_metadata(
        &self,
        signer_id: &SignerIdentifier,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<(), Error> {
        let nostr_signer = self.client.signer().await?;

        // Get signer
        let mut signer: Signer = self.storage.signer(signer_id).await?;

        if let Some(name) = name {
            signer.change_name(name);
        }

        if let Some(description) = description {
            signer.change_description(description);
        }

        // Compose and publish event
        let event: Event = v2::signer::build_event(&nostr_signer, &signer).await?;
        self.client.send_event(event).await?;

        // Re-save signer with updated metadata
        self.storage.save_signer(*signer_id, signer).await;

        Ok(())
    }

    /// Create shared signer and **send invite** to receiver
    pub async fn share_signer<S>(
        &self,
        signer_id: &SignerIdentifier,
        receiver: PublicKey,
        message: S,
    ) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let public_key: PublicKey = self.nostr_public_key().await?;

        // Get signer
        let signer: Signer = self.get_signer_by_id(signer_id).await?;

        // Build shared signer
        let shared_signer: SharedSigner = signer.to_shared(public_key, receiver);

        // Compose invite
        let invite: SharedSignerInvite = shared_signer.to_invite(message);

        // Compose and publish event
        let event: Event = v2::signer::shared::invite::build_event(invite, receiver)?;
        self.client.send_event(event).await?;

        Ok(())
    }

    /// Get shared signer invites
    pub async fn shared_signer_invites(&self) -> Result<Vec<SharedSignerInvite>, Error> {
        let invites = self.storage.shared_signer_invites().await;
        let mut invites: Vec<SharedSignerInvite> = invites.into_values().collect();
        invites.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(invites)
    }

    /// Accept a shared signer invite
    pub async fn accept_shared_signer_invite(
        &self,
        shared_signer_id: &NostrPublicIdentifier,
    ) -> Result<(), Error> {
        let nostr_signer = self.client.signer().await?;

        // Get invite
        let invite: SharedSignerInvite =
            self.storage.shared_signer_invite(shared_signer_id).await?;

        // Compose and publish event
        let event: Event =
            v2::signer::shared::build_event(&nostr_signer, invite.shared_signer()).await?;
        self.client.send_event(event).await?;

        // Delete invite
        self.storage
            .delete_shared_signer_invite(shared_signer_id)
            .await;
        Ok(())
    }

    /// Delete a vault invite
    pub async fn delete_shared_signer_invite(
        &self,
        shared_signer_id: &NostrPublicIdentifier,
    ) -> bool {
        self.storage
            .delete_shared_signer_invite(shared_signer_id)
            .await
    }

    pub async fn delete_shared_signer(
        &self,
        shared_signer_id: &NostrPublicIdentifier,
    ) -> Result<(), Error> {
        let public_key: PublicKey = self.nostr_public_key().await?;

        let filter: Filter = Filter::new()
            .kind(SHARED_SIGNER_KIND_V2)
            .author(public_key)
            .identifier(shared_signer_id.to_string())
            .limit(1);
        let res: Vec<Event> = self
            .client
            .database()
            .query(vec![filter], Order::Desc)
            .await?;
        let shared_signer_event: &Event = res.first().ok_or(Error::NotFound)?;

        let event = EventBuilder::new(
            Kind::EventDeletion,
            "",
            [Tag::event(shared_signer_event.id)],
        );
        self.client.send_event_builder(event).await?;

        self.storage.delete_shared_signer(shared_signer_id).await;

        Ok(())
    }

    pub async fn share_signer_to_multiple_public_keys(
        &self,
        signer_id: &SignerIdentifier,
        receivers: Vec<PublicKey>,
    ) -> Result<(), Error> {
        if receivers.is_empty() {
            return Err(Error::NotEnoughPublicKeys);
        }

        let public_key: PublicKey = self.nostr_public_key().await?;
        let signer: Signer = self.get_signer_by_id(signer_id).await?;

        for receiver in receivers.into_iter() {
            let _shared_signer: SharedSigner = signer.as_shared(public_key, receiver);

            todo!();

            // TODO: use send_batch_event method from nostr-sdk
            // self.client
            // .pool()
            // .send_msg(
            //      ClientMessage::event(event),
            //      RelaySendOptions::new().skip_send_confirmation(true),
            //  )
            // .await?;
            //
            // self.storage
            // .save_my_shared_signer(signer_id, event_id, public_key)
            // .await;
        }

        Ok(())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn shared_signers(&self) -> Result<Vec<GetSharedSigner>, Error> {
        let shared_signers = self.storage.shared_signers().await;
        let mut list = Vec::with_capacity(shared_signers.len());
        for (shared_signer_id, shared_signer) in shared_signers.into_iter() {
            let profile: Profile = self
                .client
                .database()
                .profile(*shared_signer.owner())
                .await?;
            list.push(GetSharedSigner {
                shared_signer_id,
                owner: profile,
                shared_signer,
            });
        }
        list.sort();
        Ok(list)
    }

    pub async fn get_shared_signers_public_keys(
        &self,
        include_contacts: bool,
    ) -> Result<Vec<PublicKey>, Error> {
        let public_keys: HashSet<PublicKey> = self.storage.get_shared_signers_public_keys().await;
        if include_contacts {
            Ok(public_keys.into_iter().collect())
        } else {
            let public_key: PublicKey = self.nostr_public_key().await?;
            let contacts: Vec<PublicKey> = self
                .client
                .database()
                .contacts_public_keys(public_key)
                .await?;
            let contacts: HashSet<PublicKey> = contacts.into_iter().collect();
            Ok(public_keys.difference(&contacts).copied().collect())
        }
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_shared_signers_by_public_key(
        &self,
        public_key: PublicKey,
    ) -> Result<Vec<GetSharedSigner>, Error> {
        let profile: Profile = self.client.database().profile(public_key).await?;
        Ok(self
            .storage
            .get_shared_signers_by_public_key(public_key)
            .await
            .into_iter()
            .map(|(shared_signer_id, shared_signer)| GetSharedSigner {
                shared_signer_id,
                owner: profile.clone(),
                shared_signer,
            })
            .collect())
    }
}
