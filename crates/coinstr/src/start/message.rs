// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::Coinstr;

use super::screen::{GenerateMessage, OpenMessage, RestoreMessage};
use super::Stage;

#[derive(Debug, Clone)]
pub enum Message {
    View(Stage),
    Open(OpenMessage),
    Restore(RestoreMessage),
    Generate(GenerateMessage),
    OpenResult(Coinstr),
}

impl From<Message> for crate::Message {
    fn from(msg: Message) -> Self {
        Self::Start(Box::new(msg))
    }
}