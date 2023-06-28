use derive_builder::Builder;
use serde::Serialize;

use crate::arkose::ArkoseToken;

use super::{Author, Role};

#[derive(Serialize, Builder, Clone)]
pub struct Content<'a> {
    content_type: ContentText,
    parts: Vec<&'a str>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ContentText {
    Text,
}

#[derive(Serialize, Builder, Clone)]
pub struct Messages<'a> {
    id: Option<&'a str>,
    author: Author,
    content: Content<'a>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Next,
    Variant,
    Continue,
}

#[derive(Serialize, Builder)]
pub struct PatchConvoRequest<'a> {
    #[builder(setter(into, strip_option), default)]
    pub conversation_id: Option<&'a str>,
    #[builder(setter(into, strip_option), default)]
    title: Option<&'a str>,
    #[builder(setter(into, strip_option), default)]
    is_visible: Option<bool>,
}

#[derive(Builder)]
pub struct GetConvoRequest<'a> {
    #[builder(setter(into, strip_option))]
    pub conversation_id: Option<&'a str>,
    #[builder(default = "0")]
    pub offset: u32,
    #[builder(default = "20")]
    pub limit: u32,
}

#[derive(Serialize, Builder)]
pub struct PostConvoGenTitleRequest<'a> {
    message_id: &'a str,
    #[serde(skip_serializing)]
    pub conversation_id: &'a str,
}

#[derive(Serialize, Builder)]
pub struct MessageFeedbackRequest<'a> {
    message_id: &'a str,
    rating: Rating,
    conversation_id: &'a str,
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

#[derive(Serialize, Builder)]
pub struct PostConvoRequest<'a> {
    action: Action,
    messages: Vec<Messages<'a>>,
    parent_message_id: &'a str,
    model: &'a str,
    #[builder(default = "-480")]
    timezone_offset_min: i64,
    #[builder(setter(into), default)]
    conversation_id: Option<&'a str>,
    #[builder(default = "false")]
    history_and_training_disabled: bool,
    #[builder(default)]
    arkose_token: Option<&'a ArkoseToken>,
}

impl<'a> TryFrom<PostNextConvoRequest<'a>> for PostConvoRequest<'a> {
    type Error = anyhow::Error;

    fn try_from(value: PostNextConvoRequest<'a>) -> Result<Self, Self::Error> {
        let body = PostConvoRequestBuilder::default()
            .action(Action::Next)
            .parent_message_id(value.parent_message_id)
            .messages(vec![MessagesBuilder::default()
                .id(Some(value.message_id))
                .author(Author { role: Role::User })
                .content(
                    ContentBuilder::default()
                        .content_type(ContentText::Text)
                        .parts(vec![value.prompt])
                        .build()?,
                )
                .build()?])
            .model(value.model)
            .conversation_id(value.conversation_id)
            .arkose_token(value.arkose_token)
            .build()?;
        Ok(body)
    }
}

impl<'a> TryFrom<PostContinueConvoRequest<'a>> for PostConvoRequest<'a> {
    type Error = anyhow::Error;

    fn try_from(value: PostContinueConvoRequest<'a>) -> Result<Self, Self::Error> {
        let body = PostConvoRequestBuilder::default()
            .action(Action::Continue)
            .conversation_id(value.conversation_id)
            .parent_message_id(value.parent_message_id)
            .model(value.model)
            .arkose_token(value.arkose_token)
            .build()?;

        Ok(body)
    }
}

impl<'a> TryFrom<PostVaraintConvoRequest<'a>> for PostConvoRequest<'a> {
    type Error = anyhow::Error;

    fn try_from(value: PostVaraintConvoRequest<'a>) -> Result<Self, Self::Error> {
        let body = PostConvoRequestBuilder::default()
            .action(Action::Variant)
            .conversation_id(value.conversation_id)
            .parent_message_id(value.parent_message_id)
            .messages(vec![MessagesBuilder::default()
                .id(Some(value.message_id))
                .author(Author { role: Role::User })
                .content(
                    ContentBuilder::default()
                        .content_type(ContentText::Text)
                        .parts(vec![value.prompt])
                        .build()?,
                )
                .build()?])
            .model(value.model)
            .arkose_token(value.arkose_token)
            .build()?;
        Ok(body)
    }
}

#[derive(Serialize, Builder)]
pub struct PostNextConvoRequest<'a> {
    /// The conversation uses a model that usually remains the same throughout the conversation
    model: &'a str,
    /// What to ask.
    prompt: &'a str,
    /// The message ID, usually generated using str(uuid.uuid4())
    message_id: &'a str,
    /// The parent message ID must also be generated for the first time. Then get the message ID of the previous reply.
    parent_message_id: &'a str,
    /// The first conversation is off the record. It can be obtained when ChatGPT replies.
    #[builder(setter(into, strip_option), default)]
    conversation_id: Option<&'a str>,

    #[builder(setter(into), default)]
    arkose_token: Option<&'a ArkoseToken>,
}

#[derive(Serialize, Builder)]
pub struct PostContinueConvoRequest<'a> {
    /// The conversation uses a model that usually remains the same throughout the conversation
    model: &'a str,
    /// Parent message ID, the message ID of the last ChatGPT reply.
    parent_message_id: &'a str,
    /// ID of a session. conversation_id Session ID.
    conversation_id: &'a str,

    #[builder(setter(into), default)]
    arkose_token: Option<&'a ArkoseToken>,
}

#[derive(Serialize, Builder)]
pub struct PostVaraintConvoRequest<'a> {
    /// The conversation uses a model that usually remains the same throughout the conversation
    model: &'a str,
    /// What to ask.
    prompt: &'a str,
    /// ID of the message sent by the previous user.
    message_id: &'a str,
    /// ID of the parent message sent by the previous user.
    parent_message_id: &'a str,
    /// The session ID must be passed on this interface.
    conversation_id: &'a str,

    #[builder(setter(into), default)]
    arkose_token: Option<&'a ArkoseToken>,
}
