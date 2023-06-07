use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Builder, Clone)]
pub struct Author {
    pub role: String,
}

#[derive(Serialize, Deserialize, Builder, Clone)]
pub struct Content {
    pub content_type: String,
    pub parts: Vec<String>,
}

#[derive(Serialize, Deserialize, Builder, Clone)]
pub struct Messages {
    pub id: Option<String>,
    pub author: Author,
    pub content: Content,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Action {
    Next,
    Variant,
    Continue,
}

impl ToString for Action {
    fn to_string(&self) -> String {
        match self {
            Action::Next => String::from("next"),
            Action::Variant => String::from("variant"),
            Action::Continue => String::from("continue"),
        }
    }
}

#[derive(Serialize, Deserialize, Builder)]
pub struct CreateConversationRequest {
    pub action: Action,
    pub messages: Vec<Messages>,
    pub parent_message_id: String,
    pub model: String,
    pub timezone_offset_min: i64,
    #[builder(setter(into, strip_option), default)]
    pub conversation_id: Option<String>,
    pub history_and_training_disabled: bool,
}

#[derive(Serialize, Deserialize, Builder)]
pub struct ClearConversationRequest {
    #[serde(skip_serializing)]
    #[builder(default = "String::new()")]
    pub conversation_id: String,
    is_visible: bool,
}

#[derive(Builder)]
pub struct GetConversationRequest {
    #[builder(default = "String::new()")]
    pub conversation_id: String,
    #[builder(default = "0")]
    pub offset: u32,
    #[builder(default = "20")]
    pub limit: u32,
}

#[derive(Serialize, Deserialize, Builder)]
pub struct RenameConversationRequest {
    #[serde(skip_serializing)]
    pub conversation_id: String,
    title: String,
}

#[derive(Serialize, Deserialize, Builder)]
pub struct MessageFeedbackRequest {
    message_id: String,
    rating: Rating,
    conversation_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Rating {
    ThumbsUp,
    ThumbsDown,
}

impl ToString for Rating {
    fn to_string(&self) -> String {
        match self {
            Rating::ThumbsUp => String::from("thumbsUp"),
            Rating::ThumbsDown => String::from("thumbsDown"),
        }
    }
}
