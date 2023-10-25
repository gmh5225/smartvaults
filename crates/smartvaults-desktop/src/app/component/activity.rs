// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Length};
use smartvaults_sdk::core::bdk::chain::ConfirmationTime;
use smartvaults_sdk::core::proposal::{CompletedProposal, Proposal};
use smartvaults_sdk::nostr::Timestamp;
use smartvaults_sdk::types::{GetCompletedProposal, GetProposal, GetTransaction};
use smartvaults_sdk::util::{self, format};

use crate::app::{Context, Message, Stage};
use crate::component::{rule, Badge, BadgeStyle, Button, ButtonStyle, Icon, Text};
use crate::theme::color::{GREEN, RED, YELLOW};
use crate::theme::icon::{BROWSER, CHECK, CLIPBOARD, FULLSCREEN, HOURGLASS};

pub struct Activity {
    proposals: Vec<GetProposal>,
    txs: Vec<GetTransaction>,
    hide_policy_id: bool,
}

impl Activity {
    pub fn new(proposals: Vec<GetProposal>, txs: Vec<GetTransaction>) -> Self {
        Self {
            proposals,
            txs,
            hide_policy_id: false,
        }
    }

    pub fn hide_policy_id(self) -> Self {
        Self {
            hide_policy_id: true,
            ..self
        }
    }

    pub fn view(self, ctx: &Context) -> Column<'static, Message> {
        let mut activities = Column::new()
            .push(
                Row::new()
                    .push(Space::with_width(Length::Fixed(70.0)))
                    .push(if self.hide_policy_id {
                        Text::new("").view()
                    } else {
                        Text::new("Vault ID")
                            .bold()
                            .width(Length::Fixed(115.0))
                            .view()
                    })
                    .push(
                        Text::new("Date/Time")
                            .bold()
                            .width(Length::Fixed(225.0))
                            .view(),
                    )
                    .push(
                        Text::new("Status")
                            .bold()
                            .width(Length::Fixed(140.0))
                            .view(),
                    )
                    .push(Text::new("Amount").bold().width(Length::Fill).view())
                    .push(
                        Text::new("Description")
                            .bold()
                            .width(Length::FillPortion(2))
                            .view(),
                    )
                    .push(Space::with_width(Length::Fixed(40.0)))
                    .push(Space::with_width(Length::Fixed(40.0)))
                    .push(Space::with_width(Length::Fixed(40.0)))
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .width(Length::Fill),
            )
            .push(rule::horizontal_bold())
            .width(Length::Fill)
            .spacing(10);

        if self.proposals.is_empty() && self.txs.is_empty() {
            activities = activities.push(Text::new("No activity").extra_light().view());
        } else {
            // Proposals
            for GetProposal {
                proposal_id,
                policy_id,
                proposal,
                signed,
            } in self.proposals.into_iter()
            {
                let row = match proposal {
                    Proposal::Spending {
                        amount,
                        description,
                        ..
                    } => Row::new()
                        .push(Space::with_width(Length::Fixed(70.0)))
                        .push(if self.hide_policy_id {
                            Text::new("").view()
                        } else {
                            Text::new(util::cut_event_id(policy_id))
                                .width(Length::Fixed(115.0))
                                .on_press(Message::View(Stage::Vault(policy_id)))
                                .view()
                        })
                        .push(Text::new("-").width(Length::Fixed(225.0)).view())
                        .push(
                            Row::new()
                                .push(
                                    Badge::new(
                                        Text::new(if signed {
                                            "To broadcast"
                                        } else {
                                            "To approve"
                                        })
                                        .small()
                                        .extra_light()
                                        .view(),
                                    )
                                    .style(if signed {
                                        BadgeStyle::Warning
                                    } else {
                                        BadgeStyle::Info
                                    })
                                    .width(Length::Fixed(125.0)),
                                )
                                .width(Length::Fixed(140.0)),
                        )
                        .push(
                            Text::new(format!(
                                "{} sat",
                                if ctx.hide_balances {
                                    String::from("*****")
                                } else {
                                    format!("-{}", format::number(amount))
                                }
                            ))
                            .color(RED)
                            .width(Length::Fill)
                            .view(),
                        )
                        .push(Text::new(description).width(Length::FillPortion(2)).view())
                        .push(Space::with_width(Length::Fixed(40.0)))
                        .push(Space::with_width(Length::Fixed(40.0)))
                        .push(
                            Button::new()
                                .icon(FULLSCREEN)
                                .on_press(Message::View(Stage::Proposal(proposal_id)))
                                .width(Length::Fixed(40.0))
                                .view(),
                        )
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                    Proposal::ProofOfReserve { message, .. } => Row::new()
                        .push(Space::with_width(Length::Fixed(70.0)))
                        .push(if self.hide_policy_id {
                            Text::new("").view()
                        } else {
                            Text::new(util::cut_event_id(policy_id))
                                .width(Length::Fixed(115.0))
                                .on_press(Message::View(Stage::Vault(policy_id)))
                                .view()
                        })
                        .push(Text::new("-").width(Length::Fixed(125.0)).view())
                        .push(
                            Row::new()
                                .push(
                                    Badge::new(
                                        Text::new(if signed {
                                            "To broadcast"
                                        } else {
                                            "To approve"
                                        })
                                        .small()
                                        .extra_light()
                                        .view(),
                                    )
                                    .style(if signed {
                                        BadgeStyle::Warning
                                    } else {
                                        BadgeStyle::Info
                                    })
                                    .width(Length::Fixed(125.0)),
                                )
                                .width(Length::Fixed(140.0)),
                        )
                        .push(Text::new("-").width(Length::Fill).view())
                        .push(Text::new(message).width(Length::FillPortion(2)).view())
                        .push(Space::with_width(Length::Fixed(40.0)))
                        .push(Space::with_width(Length::Fixed(40.0)))
                        .push(
                            Button::new()
                                .icon(FULLSCREEN)
                                .on_press(Message::View(Stage::Proposal(proposal_id)))
                                .width(Length::Fixed(40.0))
                                .view(),
                        )
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                };
                activities = activities.push(row).push(rule::horizontal());
            }

            // Transactions
            for GetTransaction {
                policy_id,
                tx,
                label,
                block_explorer,
            } in self.txs.into_iter()
            {
                let status = if tx.confirmation_time.is_confirmed() {
                    Icon::new(CHECK).color(GREEN)
                } else {
                    Icon::new(HOURGLASS).color(YELLOW)
                };

                let (total, positive): (u64, bool) = {
                    let received: i64 = tx.received as i64;
                    let sent: i64 = tx.sent as i64;
                    let tot = received - sent;
                    let positive = tot >= 0;
                    (tot.unsigned_abs(), positive)
                };

                let row = Row::new()
                    .push(status.width(Length::Fixed(70.0)))
                    .push(if self.hide_policy_id {
                        Text::new("").view()
                    } else {
                        Text::new(util::cut_event_id(policy_id))
                            .width(Length::Fixed(115.0))
                            .on_press(Message::View(Stage::Vault(policy_id)))
                            .view()
                    })
                    .push(
                        Text::new(if ctx.hide_balances {
                            String::from("*****")
                        } else {
                            match tx.confirmation_time {
                                ConfirmationTime::Confirmed { time, .. } => {
                                    Timestamp::from(time).to_human_datetime()
                                }
                                ConfirmationTime::Unconfirmed { .. } => String::from("Pending"),
                            }
                        })
                        .width(Length::Fixed(225.0))
                        .view(),
                    )
                    .push(
                        Row::new()
                            .push(
                                Badge::new(Text::new("Completed").small().extra_light().view())
                                    .style(BadgeStyle::Success)
                                    .width(Length::Fixed(125.0)),
                            )
                            .width(Length::Fixed(140.0)),
                    )
                    .push(
                        Text::new(format!(
                            "{} sat",
                            if ctx.hide_balances {
                                String::from("*****")
                            } else {
                                format!(
                                    "{}{}",
                                    if positive { "+" } else { "-" },
                                    format::number(total)
                                )
                            }
                        ))
                        .color(if positive { GREEN } else { RED })
                        .width(Length::Fill)
                        .view(),
                    )
                    .push(
                        Text::new(label.unwrap_or_default())
                            .width(Length::FillPortion(2))
                            .view(),
                    )
                    .push(
                        Button::new()
                            .icon(CLIPBOARD)
                            .style(ButtonStyle::Bordered)
                            .on_press(Message::Clipboard(tx.txid().to_string()))
                            .width(Length::Fixed(40.0))
                            .view(),
                    )
                    .push({
                        let mut btn = Button::new()
                            .icon(BROWSER)
                            .style(ButtonStyle::Bordered)
                            .width(Length::Fixed(40.0));

                        if let Some(url) = block_explorer {
                            btn = btn.on_press(Message::OpenInBrowser(url));
                        }

                        btn.view()
                    })
                    .push(
                        Button::new()
                            .icon(FULLSCREEN)
                            .on_press(Message::View(Stage::Transaction {
                                policy_id,
                                txid: tx.txid(),
                            }))
                            .width(Length::Fixed(40.0))
                            .view(),
                    )
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .width(Length::Fill);
                activities = activities.push(row).push(rule::horizontal());
            }
        }

        activities
    }
}

pub struct CompletedProposalsList {
    map: Vec<GetCompletedProposal>,
}

impl CompletedProposalsList {
    pub fn new(map: Vec<GetCompletedProposal>) -> Self {
        Self { map }
    }

    pub fn view(self) -> Column<'static, Message> {
        let mut proposals = Column::new()
            .push(
                Row::new()
                    .push(Text::new("ID").bold().width(Length::Fixed(115.0)).view())
                    .push(
                        Text::new("Vault ID")
                            .bold()
                            .width(Length::Fixed(115.0))
                            .view(),
                    )
                    .push(Text::new("Type").bold().width(Length::Fixed(125.0)).view())
                    .push(Text::new("Description").bold().width(Length::Fill).view())
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .width(Length::Fill),
            )
            .push(rule::horizontal_bold())
            .width(Length::Fill)
            .spacing(10);

        if self.map.is_empty() {
            proposals = proposals.push(Text::new("No proposals").extra_light().view());
        } else {
            for GetCompletedProposal {
                policy_id,
                completed_proposal_id,
                proposal,
            } in self.map.iter()
            {
                let row = match proposal {
                    CompletedProposal::Spending { description, .. } => Row::new()
                        .push(
                            Text::new(util::cut_event_id(*completed_proposal_id))
                                .width(Length::Fixed(115.0))
                                .view(),
                        )
                        .push(
                            Text::new(util::cut_event_id(*policy_id))
                                .width(Length::Fixed(115.0))
                                .on_press(Message::View(Stage::Vault(*policy_id)))
                                .view(),
                        )
                        .push(Text::new("spending").width(Length::Fixed(125.0)).view())
                        .push(Text::new(description).width(Length::Fill).view())
                        .push(
                            Button::new()
                                .icon(FULLSCREEN)
                                .on_press(Message::View(Stage::CompletedProposal(
                                    *completed_proposal_id,
                                )))
                                .width(Length::Fixed(40.0))
                                .view(),
                        )
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                    CompletedProposal::ProofOfReserve { message, .. } => Row::new()
                        .push(
                            Text::new(util::cut_event_id(*completed_proposal_id))
                                .width(Length::Fixed(115.0))
                                .view(),
                        )
                        .push(
                            Text::new(util::cut_event_id(*policy_id))
                                .width(Length::Fixed(115.0))
                                .on_press(Message::View(Stage::Vault(*policy_id)))
                                .view(),
                        )
                        .push(
                            Text::new("proof-of-reserve")
                                .width(Length::Fixed(125.0))
                                .view(),
                        )
                        .push(Text::new(message).width(Length::Fill).view())
                        .push(
                            Button::new()
                                .icon(FULLSCREEN)
                                .on_press(Message::View(Stage::CompletedProposal(
                                    *completed_proposal_id,
                                )))
                                .width(Length::Fixed(40.0))
                                .view(),
                        )
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                };
                proposals = proposals.push(row).push(rule::horizontal());
            }
        }

        proposals
    }
}