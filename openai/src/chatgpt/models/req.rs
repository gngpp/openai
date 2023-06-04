use derive_builder::Builder;
use serde::{Deserialize, Serialize};

use crate::chatgpt::ToConversationID;

#[derive(Serialize, Deserialize, Clone)]
pub struct Author {
    pub role: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Content {
    pub content_type: String,
    pub parts: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Messages {
    pub id: String,
    pub author: Author,
    pub content: Content,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Action {
    Next,
}

impl ToString for Action {
    fn to_string(&self) -> String {
        match self {
            Action::Next => String::from("next"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Builder)]
pub struct CreateConversationRequest {
    pub action: Action,
    pub messages: Vec<Messages>,
    pub parent_message_id: String,
    pub model: String,
    pub timezone_offset_min: i64,
    pub history_and_training_disabled: bool,
}

#[derive(Serialize, Deserialize, Builder)]
pub struct DeleteConversationRequest {
    #[serde(skip_serializing)]
    conversation_id: String,
    is_visible: bool,
}

impl ToConversationID for DeleteConversationRequest {
    fn to_conversation_id(&self) -> &str {
        &self.conversation_id
    }
}

#[derive(Serialize, Deserialize, Builder)]
pub struct RenameConversationRequest {
    #[serde(skip_serializing)]
    conversation_id: String,
    title: String,
}

impl ToConversationID for RenameConversationRequest {
    fn to_conversation_id(&self) -> &str {
        &self.conversation_id
    }
}
