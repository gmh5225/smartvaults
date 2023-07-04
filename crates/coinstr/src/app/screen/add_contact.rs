// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::nostr::prelude::FromPkStr;
use coinstr_sdk::nostr::Keys;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, Text, TextInput};
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
    loading: bool,
    error: Option<String>,
}

impl AddContactState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for AddContactState {
    fn title(&self) -> String {
        String::from("Add contact")
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::AddContact(msg) = message {
            match msg {
                AddContactMessage::PublicKeyChanged(public_key) => self.public_key = public_key,
                AddContactMessage::ErrorChanged(error) => {
                    self.error = error;
                    self.loading = false;
                }
                AddContactMessage::SaveContact => {
                    let client = ctx.client.clone();
                    match Keys::from_pk_str(&self.public_key) {
                        Ok(keys) => {
                            self.loading = true;
                            return Command::perform(
                                async move { client.add_contact(keys.public_key()).await },
                                |res| match res {
                                    Ok(_) => Message::View(Stage::Contacts),
                                    Err(e) => {
                                        AddContactMessage::ErrorChanged(Some(e.to_string())).into()
                                    }
                                },
                            );
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

        let mut save_contact_btn = button::primary("Save contact").width(Length::Fill);

        if !self.loading {
            save_contact_btn = save_contact_btn.on_press(AddContactMessage::SaveContact.into());
        }

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
