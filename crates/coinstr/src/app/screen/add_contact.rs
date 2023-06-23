// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::str::FromStr;

use coinstr_sdk::core::bitcoin::XOnlyPublicKey;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, Text, TextInput};
use crate::constants::APP_NAME;
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum AddContactMessage {
    PublicKeyChanged(String),
    ErrorChanged(Option<String>),
    SaveContact,
}

#[derive(Debug, Default)]
pub struct AddContactState {
    public_key: String,
    error: Option<String>,
}

impl AddContactState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for AddContactState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Add contact")
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::AddContact(msg) = message {
            match msg {
                AddContactMessage::PublicKeyChanged(public_key) => self.public_key = public_key,
                AddContactMessage::ErrorChanged(error) => self.error = error,
                AddContactMessage::SaveContact => {
                    let client = ctx.client.clone();
                    match XOnlyPublicKey::from_str(&self.public_key) {
                        Ok(public_key) => {
                            return Command::perform(
                                async move { client.add_contact(public_key).await },
                                |res| match res {
                                    Ok(_) => Message::View(Stage::Contacts),
                                    Err(e) => {
                                        AddContactMessage::ErrorChanged(Some(e.to_string())).into()
                                    }
                                },
                            )
                        }
                        Err(e) => self.error = Some(e.to_string()),
                    }
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let public_key = TextInput::new("Public Key", &self.public_key)
            .on_input(|s| AddContactMessage::PublicKeyChanged(s).into())
            .placeholder("Public Key")
            .view();

        let error = if let Some(error) = &self.error {
            Row::new().push(Text::new(error).color(DARK_RED).view())
        } else {
            Row::new()
        };

        let save_contact_btn = button::primary("Save contact")
            .on_press(AddContactMessage::SaveContact.into())
            .width(Length::Fill);

        let content = Column::new()
            .push(
                Column::new()
                    .push(Text::new("Add contact").size(24).bold().view())
                    .push(Text::new("Add a new contact").extra_light().view())
                    .spacing(10)
                    .width(Length::Fill),
            )
            .push(public_key)
            .push(error)
            .push(Space::with_height(Length::Fixed(15.0)))
            .push(save_contact_btn)
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20)
            .max_width(400);

        Dashboard::new().view(ctx, content, true, true)
    }
}

impl From<AddContactState> for Box<dyn State> {
    fn from(s: AddContactState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<AddContactMessage> for Message {
    fn from(msg: AddContactMessage) -> Self {
        Self::AddContact(msg)
    }
}