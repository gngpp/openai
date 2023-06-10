use derive_builder::Builder;
use serde::Serialize;

use super::{Author, Role};

#[derive(Serialize, Builder, Clone)]
pub struct Content {
    content_type: ContentText,
    parts: Vec<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ContentText {
    Text,
}

#[derive(Serialize, Builder, Clone)]
pub struct Messages {
    id: Option<String>,
    author: Author,
    content: Content,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Next,
    Variant,
    Continue,
}

#[derive(Serialize, Builder)]
pub struct PostConversationBody {
    action: Action,
    messages: Vec<Messages>,
    parent_message_id: String,
    model: String,
    #[builder(default = "-480")]
    timezone_offset_min: i64,
    #[builder(setter(into, strip_option), default)]
    conversation_id: Option<String>,
    #[builder(default = "false")]
    history_and_training_disabled: bool,
}

impl TryFrom<PostConversationRequest> for PostConversationBody {
    type Error = anyhow::Error;

    fn try_from(value: PostConversationRequest) -> Result<Self, Self::Error> {
        match value {
            PostConversationRequest::Next(v) => v.try_into(),
            PostConversationRequest::Variant(v) => v.try_into(),
            PostConversationRequest::Continue(v) => v.try_into(),
        }
    }
}

#[derive(Serialize, Builder)]
pub struct PatchConversationRequest {
    #[builder(default = "String::new()")]
    pub conversation_id: String,
    #[builder(setter(into, strip_option), default)]
    title: Option<String>,
    #[builder(setter(into, strip_option), default)]
    is_visible: Option<bool>,
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

#[derive(Serialize, Builder)]
pub struct MessageFeedbackRequest {
    message_id: String,
    rating: Rating,
    conversation_id: String,
}

#[derive(Serialize, Clone)]
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

pub enum PostConversationRequest {
    Next(PostNextConversationBody),
    Variant(PostVaraintConversationBody),
    Continue(PostContinueConversationBody),
}

#[derive(Serialize, Builder)]
pub struct PostNextConversationBody {
    model: String,
    prompt: String,
}

impl TryInto<PostConversationBody> for PostNextConversationBody {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<PostConversationBody, Self::Error> {
        let message_id = uuid::Uuid::new_v4();
        let parent_message_id = uuid::Uuid::new_v4();
        let body = PostConversationBodyBuilder::default()
            .action(Action::Next)
            .parent_message_id(parent_message_id.to_string())
            .messages(vec![MessagesBuilder::default()
                .id(Some(message_id.to_string()))
                .author(Author { role: Role::User })
                .content(
                    ContentBuilder::default()
                        .content_type(ContentText::Text)
                        .parts(vec![self.prompt])
                        .build()?,
                )
                .build()?])
            .model(self.model)
            .build()?;
        Ok(body)
    }
}

#[derive(Serialize, Builder)]
pub struct PostContinueConversationBody {
    model: String,
    parent_message_id: String,
    conversation_id: String,
}

impl TryInto<PostConversationBody> for PostContinueConversationBody {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<PostConversationBody, Self::Error> {
        let body = PostConversationBodyBuilder::default()
            .action(Action::Continue)
            .conversation_id(self.conversation_id)
            .parent_message_id(self.parent_message_id)
            .model(self.model)
            .build()?;

        Ok(body)
    }
}

#[derive(Serialize, Builder)]
pub struct PostVaraintConversationBody {
    model: String,
    prompt: String,
    message_id: String,
    parent_message_id: String,
    conversation_id: String,
}

impl TryInto<PostConversationBody> for PostVaraintConversationBody {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<PostConversationBody, Self::Error> {
        let body = PostConversationBodyBuilder::default()
            .action(Action::Variant)
            .parent_message_id(self.parent_message_id)
            .messages(vec![MessagesBuilder::default()
                .id(Some(self.message_id))
                .author(Author { role: Role::User })
                .content(
                    ContentBuilder::default()
                        .content_type(ContentText::Text)
                        .parts(vec![self.prompt])
                        .build()?,
                )
                .build()?])
            .model(self.model)
            .build()?;
        Ok(body)
    }
}
