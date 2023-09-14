// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use iced_aw::{Card, Modal};
use rfd::FileDialog;
use smartvaults_sdk::core::proposal::Proposal;
use smartvaults_sdk::core::signer::{Signer, SignerType};
use smartvaults_sdk::core::types::Psbt;
use smartvaults_sdk::core::CompletedProposal;
use smartvaults_sdk::nostr::prelude::psbt::PartiallySignedTransaction;
use smartvaults_sdk::nostr::EventId;
use smartvaults_sdk::types::{GetApproval, GetProposal};
use smartvaults_sdk::util;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, Text};
use crate::theme::color::{GREEN, RED, YELLOW};
use crate::theme::icon::{CLIPBOARD, SAVE, TRASH};

#[derive(Debug, Clone)]
pub enum ProposalMessage {
    LoadProposal(Proposal, bool, EventId, Vec<GetApproval>, Option<Signer>),
    Approve,
    Finalize,
    Signed(bool),
    Reload,
    ExportPsbt,
    RevokeApproval(EventId),
    AskDeleteConfirmation,
    Delete,
    ErrorChanged(Option<String>),
    CloseModal,
}

#[derive(Debug)]
pub struct ProposalState {
    loading: bool,
    loaded: bool,
    show_modal: bool,
    signed: bool,
    proposal_id: EventId,
    proposal: Option<Proposal>,
    policy_id: Option<EventId>,
    approved_proposals: Vec<GetApproval>,
    signer: Option<Signer>,
    error: Option<String>,
}

impl ProposalState {
    pub fn new(proposal_id: EventId) -> Self {
        Self {
            loading: false,
            loaded: false,
            show_modal: false,
            signed: false,
            proposal_id,
            proposal: None,
            policy_id: None,
            approved_proposals: Vec::new(),
            signer: None,
            error: None,
        }
    }
}

impl State for ProposalState {
    fn title(&self) -> String {
        format!("Proposal #{}", util::cut_event_id(self.proposal_id))
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        if self.loading {
            return Command::none();
        }

        let client = ctx.client.clone();
        let proposal_id = self.proposal_id;
        self.loading = true;
        Command::perform(
            async move {
                let GetProposal {
                    policy_id,
                    proposal,
                    signed,
                    ..
                } = client.get_proposal_by_id(proposal_id).await.ok()?;
                let signer = client
                    .search_signer_by_descriptor(proposal.descriptor())
                    .await
                    .ok();
                let approvals = client
                    .get_approvals_by_proposal_id(proposal_id)
                    .await
                    .unwrap_or_default();

                Some((proposal, signed, policy_id, approvals, signer))
            },
            |res| match res {
                Some((proposal, signed, policy_id, approvals, signer)) => {
                    ProposalMessage::LoadProposal(proposal, signed, policy_id, approvals, signer)
                        .into()
                }
                None => Message::View(Stage::Dashboard),
            },
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Proposal(msg) = message {
            match msg {
                ProposalMessage::LoadProposal(proposal, signed, policy_id, approvals, signer) => {
                    self.proposal = Some(proposal);
                    self.policy_id = Some(policy_id);
                    self.signed = signed;
                    self.approved_proposals = approvals;
                    self.signer = signer;
                    self.loading = false;
                    self.loaded = true;
                }
                ProposalMessage::ErrorChanged(error) => {
                    self.loading = false;
                    self.error = error;
                }
                ProposalMessage::Approve => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let proposal_id = self.proposal_id;
                    let signer = self.signer.clone();
                    return Command::perform(
                        async move {
                            match signer {
                                Some(signer) => match signer.signer_type() {
                                    SignerType::Seed => {
                                        client.approve(proposal_id).await?;
                                    }
                                    SignerType::Hardware => {
                                        //client.approve_with_hwi_signer(proposal_id, signer).await?;
                                    }
                                    SignerType::AirGap => {
                                        let path = FileDialog::new()
                                            .set_title("Select signed PSBT")
                                            .pick_file();

                                        if let Some(path) = path {
                                            let signed_psbt =
                                                PartiallySignedTransaction::from_file(path)?;
                                            client
                                                .approve_with_signed_psbt(proposal_id, signed_psbt)
                                                .await?;
                                        }
                                    }
                                },
                                None => {
                                    client.approve(proposal_id).await?;
                                }
                            };
                            Ok::<(), Box<dyn std::error::Error>>(())
                        },
                        |res| match res {
                            Ok(_) => ProposalMessage::Reload.into(),
                            Err(e) => ProposalMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                ProposalMessage::Finalize => {
                    self.loading = true;

                    let client = ctx.client.clone();
                    let proposal_id = self.proposal_id;

                    if let Some(policy_id) = self.policy_id {
                        return Command::perform(
                            async move { client.finalize(proposal_id).await },
                            move |res| match res {
                                Ok(proposal) => match proposal {
                                    CompletedProposal::Spending { tx, .. } => {
                                        Message::View(Stage::Transaction {
                                            policy_id,
                                            txid: tx.txid(),
                                        })
                                    }
                                    CompletedProposal::ProofOfReserve { .. } => {
                                        Message::View(Stage::History)
                                    }
                                },
                                Err(e) => ProposalMessage::ErrorChanged(Some(e.to_string())).into(),
                            },
                        );
                    } else {
                        self.error = Some(String::from("No policy id found"));
                    }
                }
                ProposalMessage::Signed(value) => self.signed = value,
                ProposalMessage::Reload => {
                    self.loading = false;
                    return self.load(ctx);
                }
                ProposalMessage::ExportPsbt => {
                    if let Some(proposal) = &self.proposal {
                        let path = FileDialog::new()
                            .set_title("Export PSBT")
                            .set_file_name(&format!(
                                "proposal-{}.psbt",
                                util::cut_event_id(self.proposal_id)
                            ))
                            .save_file();

                        if let Some(path) = path {
                            let psbt = proposal.psbt();
                            match psbt.save_to_file(&path) {
                                Ok(_) => {
                                    tracing::info!("PSBT exported to {}", path.display())
                                }
                                Err(e) => tracing::error!("Impossible to create file: {e}"),
                            }
                        }
                    }
                }
                ProposalMessage::RevokeApproval(approval_id) => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.revoke_approval(approval_id).await },
                        |res| match res {
                            Ok(_) => ProposalMessage::Reload.into(),
                            Err(e) => ProposalMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                ProposalMessage::AskDeleteConfirmation => self.show_modal = true,
                ProposalMessage::Delete => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let proposal_id = self.proposal_id;
                    return Command::perform(
                        async move { client.delete_proposal_by_id(proposal_id).await },
                        |res| match res {
                            Ok(_) => Message::View(Stage::Proposals),
                            Err(e) => ProposalMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                ProposalMessage::CloseModal => self.show_modal = false,
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);

        if self.loaded {
            if let Some(proposal) = &self.proposal {
                if let Some(policy_id) = self.policy_id {
                    content = content
                        .push(
                            Text::new(format!(
                                "Proposal #{}",
                                util::cut_event_id(self.proposal_id)
                            ))
                            .size(40)
                            .bold()
                            .view(),
                        )
                        .push(Space::with_height(Length::Fixed(40.0)))
                        .push(
                            Text::new(format!("Policy ID: {}", util::cut_event_id(policy_id)))
                                .on_press(Message::View(Stage::Policy(policy_id)))
                                .view(),
                        );

                    let finalize_btn_text: &str = match proposal {
                        Proposal::Spending {
                            to_address,
                            amount,
                            description,
                            psbt,
                            ..
                        } => {
                            content = content
                                .push(Text::new("Type: spending").view())
                                .push(
                                    Text::new(format!(
                                        "Address: {}",
                                        to_address.clone().assume_checked()
                                    ))
                                    .view(),
                                )
                                .push(
                                    Text::new(format!(
                                        "Amount: {} sat",
                                        util::format::number(*amount)
                                    ))
                                    .view(),
                                );

                            match psbt.fee() {
                                Ok(fee) => {
                                    content = content.push(
                                        Text::new(format!(
                                            "Fee: {} sat",
                                            util::format::number(fee.to_sat())
                                        ))
                                        .view(),
                                    )
                                }
                                Err(e) => {
                                    tracing::error!("Impossible to calculate fee: {e}");
                                }
                            };

                            if !description.is_empty() {
                                content = content
                                    .push(Text::new(format!("Description: {description}")).view());
                            }

                            "Broadcast"
                        }
                        Proposal::ProofOfReserve { message, .. } => {
                            content = content
                                .push(Text::new("Type: proof-of-reserve").view())
                                .push(Text::new(format!("Message: {message}")).view());

                            "Finalize"
                        }
                    };

                    let mut status = Row::new().push(Text::new("Status: ").view());

                    if self.signed {
                        status = status.push(Text::new("signed").color(GREEN).view());
                    } else {
                        status = status.push(Text::new("unsigned").color(YELLOW).view());
                    }

                    content = content.push(status).push(
                        Text::new(format!(
                            "Signer: {}",
                            self.signer
                                .as_ref()
                                .map(|s| s.to_string())
                                .unwrap_or_else(|| String::from("Unknown"))
                        ))
                        .view(),
                    );

                    let (approve_btn, mut finalize_btn) =
                        match self
                            .approved_proposals
                            .iter()
                            .find(|GetApproval { user, .. }| {
                                user.public_key() == ctx.client.keys().public_key()
                            }) {
                            Some(_) => {
                                let approve_btn =
                                    Button::new().style(ButtonStyle::Bordered).text("Approve");
                                let finalize_btn = Button::new().text(finalize_btn_text);
                                (approve_btn, finalize_btn)
                            }
                            None => {
                                let approve_btn = Button::new()
                                    .text("Approve")
                                    .on_press(ProposalMessage::Approve.into())
                                    .loading(self.loading);
                                let finalize_btn = Button::new()
                                    .style(ButtonStyle::Bordered)
                                    .text(finalize_btn_text);

                                (approve_btn, finalize_btn)
                            }
                        };

                    if self.signed && !self.loading {
                        finalize_btn = finalize_btn.on_press(ProposalMessage::Finalize.into());
                    }

                    let export_btn = Button::new()
                        .style(ButtonStyle::Bordered)
                        .icon(SAVE)
                        .text("Export PSBT")
                        .on_press(ProposalMessage::ExportPsbt.into())
                        .loading(self.loading)
                        .view();
                    let copy_psbt = Button::new()
                        .style(ButtonStyle::Bordered)
                        .icon(CLIPBOARD)
                        .text("Copy PSBT")
                        .on_press(Message::Clipboard(proposal.psbt().as_base64()))
                        .view();
                    let delete_btn = Button::new()
                        .style(ButtonStyle::Danger)
                        .icon(TRASH)
                        .text("Delete")
                        .on_press(ProposalMessage::AskDeleteConfirmation.into())
                        .loading(self.loading)
                        .view();

                    content = content
                        .push(Space::with_height(10.0))
                        .push(
                            Row::new()
                                .push(approve_btn.view())
                                .push(finalize_btn.view())
                                .push(export_btn)
                                .push(copy_psbt)
                                .push(delete_btn)
                                .spacing(10),
                        )
                        .push(Space::with_height(20.0));

                    if let Some(error) = &self.error {
                        content = content.push(Text::new(error).color(RED).view());
                    };

                    if !self.approved_proposals.is_empty() {
                        content = content
                            .push(Text::new("Approvals").bold().big().view())
                            .push(Space::with_height(10.0))
                            .push(
                                Row::new()
                                    .push(
                                        Text::new("ID")
                                            .bold()
                                            .big()
                                            .width(Length::Fixed(115.0))
                                            .view(),
                                    )
                                    .push(
                                        Text::new("Date/Time")
                                            .bold()
                                            .big()
                                            .width(Length::Fill)
                                            .view(),
                                    )
                                    .push(Text::new("User").bold().big().width(Length::Fill).view())
                                    .push(Space::with_width(Length::Fixed(40.0)))
                                    .spacing(10)
                                    .align_items(Alignment::Center)
                                    .width(Length::Fill),
                            )
                            .push(rule::horizontal_bold());

                        let my_public_key = ctx.client.keys().public_key();
                        for GetApproval {
                            approval_id,
                            user,
                            timestamp,
                            ..
                        } in self.approved_proposals.iter()
                        {
                            let mut row = Row::new()
                                .push(
                                    Text::new(util::cut_event_id(*approval_id))
                                        .width(Length::Fixed(115.0))
                                        .view(),
                                )
                                .push(
                                    Text::new(timestamp.to_human_datetime())
                                        .width(Length::Fill)
                                        .view(),
                                )
                                .push(Text::new(user.name()).width(Length::Fill).view())
                                .spacing(10)
                                .align_items(Alignment::Center)
                                .width(Length::Fill);

                            if my_public_key == user.public_key() {
                                row = row.push(
                                    Button::new()
                                        .style(ButtonStyle::BorderedDanger)
                                        .icon(TRASH)
                                        .width(Length::Fixed(40.0))
                                        .on_press(
                                            ProposalMessage::RevokeApproval(*approval_id).into(),
                                        )
                                        .view(),
                                )
                            } else {
                                row = row.push(
                                    Row::new()
                                        .push(Space::with_height(Length::Fixed(40.0)))
                                        .push(Space::with_width(Length::Fixed(40.0))),
                                );
                            }
                            content = content.push(row).push(rule::horizontal());
                        }
                    }
                }
            }
        };

        let dashboard = Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, false, false);

        Modal::new(
            self.show_modal,
            dashboard,
            Card::new(
                Text::new("Delete proposal").view(),
                Text::new("Do you want really delete this proposal?").view(),
            )
            .foot(
                Row::new()
                    .spacing(10)
                    .padding(5)
                    .width(Length::Fill)
                    .push(
                        Button::new()
                            .style(ButtonStyle::BorderedDanger)
                            .text("Confirm")
                            .width(Length::Fill)
                            .on_press(ProposalMessage::Delete.into())
                            .loading(self.loading)
                            .view(),
                    )
                    .push(
                        Button::new()
                            .style(ButtonStyle::Bordered)
                            .text("Close")
                            .width(Length::Fill)
                            .on_press(ProposalMessage::CloseModal.into())
                            .view(),
                    ),
            )
            .max_width(300.0),
        )
        .on_esc(ProposalMessage::CloseModal.into())
        .backdrop(ProposalMessage::CloseModal.into())
        .into()
    }
}

impl From<ProposalState> for Box<dyn State> {
    fn from(s: ProposalState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<ProposalMessage> for Message {
    fn from(msg: ProposalMessage) -> Self {
        Self::Proposal(msg)
    }
}