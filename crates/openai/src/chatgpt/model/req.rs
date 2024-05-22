use crate::gpt_model::GPTModel;

use super::{Author, Role};
use serde::Serialize;
use typed_builder::TypedBuilder;

#[derive(Serialize, TypedBuilder)]
pub struct Content<'a> {
    content_type: ContentText,
    parts: Vec<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ContentText {
    Text,
}

#[derive(Serialize, TypedBuilder)]
pub struct Messages<'a> {
    id: String,
    author: Author,
    content: Content<'a>,
    metadata: Metadata,
}

#[derive(Serialize)]
pub struct Metadata {}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Next,
    Variant,
    Continue,
}

#[derive(Serialize, TypedBuilder)]
pub struct ConversationMode<'a> {
    pub kind: &'a str,
    pub gizmo_id:Option<&'a str>,
}

#[derive(Serialize, TypedBuilder)]
pub struct PatchConvoRequest<'a> {
    #[builder(setter(into, strip_option), default)]
    pub conversation_id: Option<&'a str>,
    #[builder(setter(into, strip_option), default)]
    title: Option<&'a str>,
    #[builder(setter(into, strip_option), default)]
    is_visible: Option<bool>,
}

#[derive(TypedBuilder)]
pub struct GetConvoRequest<'a> {
    #[builder(setter(into, strip_option))]
    pub conversation_id: Option<&'a str>,
    #[builder(default = 0)]
    pub offset: u32,
    #[builder(default = 20)]
    pub limit: u32,
}

#[derive(Serialize, TypedBuilder)]
pub struct PostConvoGenTitleRequest<'a> {
    message_id: &'a str,
    #[serde(skip_serializing)]
    pub conversation_id: &'a str,
}

#[derive(Serialize, TypedBuilder)]
pub struct MessageFeedbackRequest<'a> {
    message_id: &'a str,
    rating: Rating,
    conversation_id: &'a str,
}

#[derive(Serialize)]
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

#[derive(Serialize, TypedBuilder)]
pub struct PostConvoRequest<'a> {
    action: Action,
    messages: Vec<Messages<'a>>,
    parent_message_id: &'a str,
    model: GPTModel,
    #[builder(default = -480)]
    timezone_offset_min: i64,
    #[builder(setter(into), default)]
    conversation_id: Option<&'a str>,
    #[builder(default = false)]
    history_and_training_disabled: bool,
    #[builder(setter(into), default)]
    conversation_mode: Option<ConversationMode<'a>>,
    #[builder(default = false)]
    force_paragen: bool,
    #[builder(default = false)]
    force_rate_limit: bool,
    #[builder(default = vec![])]
    suggestions: Vec<&'a str>,
    #[builder(setter(into), default)]
    arkose_token: Option<&'a str>,
}

impl<'a> From<PostNextConvoRequest<'a>> for PostConvoRequest<'a> {
    fn from(value: PostNextConvoRequest<'a>) -> Self {
        PostConvoRequest::builder()
            .action(Action::Next)
            .parent_message_id(value.parent_message_id)
            .messages(vec![Messages::builder()
                .id(value.message_id.to_owned())
                .author(Author { role: Role::User })
                .content(
                    Content::builder()
                        .content_type(ContentText::Text)
                        .parts(vec![value.prompt])
                        .build(),
                )
                .metadata(Metadata {})
                .build()])
            .model(value.model)
            .conversation_id(value.conversation_id)
            .arkose_token(value.arkose_token)
            .build()
    }
}

impl<'a> From<PostContinueConvoRequest<'a>> for PostConvoRequest<'a> {
    fn from(value: PostContinueConvoRequest<'a>) -> Self {
        PostConvoRequest::builder()
            .action(Action::Continue)
            .conversation_id(value.conversation_id)
            .parent_message_id(value.parent_message_id)
            .messages(vec![])
            .model(value.model)
            .arkose_token(value.arkose_token)
            .build()
    }
}

impl<'a> From<PostVaraintConvoRequest<'a>> for PostConvoRequest<'a> {
    fn from(value: PostVaraintConvoRequest<'a>) -> Self {
        PostConvoRequest::builder()
            .action(Action::Variant)
            .conversation_id(value.conversation_id)
            .parent_message_id(value.parent_message_id)
            .messages(vec![Messages::builder()
                .id(value.message_id.to_owned())
                .author(Author { role: Role::User })
                .content(
                    Content::builder()
                        .content_type(ContentText::Text)
                        .parts(vec![value.prompt])
                        .build(),
                )
                .metadata(Metadata {})
                .build()])
            .model(value.model)
            .arkose_token(value.arkose_token)
            .build()
    }
}

#[derive(Serialize, TypedBuilder)]
pub struct PostNextConvoRequest<'a> {
    /// The conversation uses a model that usually remains the same throughout the conversation
    model: GPTModel,
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
    arkose_token: Option<&'a str>,
}

#[derive(Serialize, TypedBuilder)]
pub struct PostContinueConvoRequest<'a> {
    /// The conversation uses a model that usually remains the same throughout the conversation
    model: GPTModel,
    /// Parent message ID, the message ID of the last ChatGPT reply.
    parent_message_id: &'a str,
    /// ID of a session. conversation_id Session ID.
    conversation_id: &'a str,

    #[builder(setter(into), default)]
    arkose_token: Option<&'a str>,
}

#[derive(Serialize, TypedBuilder)]
pub struct PostVaraintConvoRequest<'a> {
    /// The conversation uses a model that usually remains the same throughout the conversation
    model: GPTModel,
    /// What to ask.
    prompt: &'a str,
    /// ID of the message sent by the previous user.
    message_id: &'a str,
    /// ID of the parent message sent by the previous user.
    parent_message_id: &'a str,
    /// The session ID must be passed on this interface.
    conversation_id: &'a str,

    #[builder(setter(into), default)]
    arkose_token: Option<&'a str>,
}
