use derive_builder::Builder;
use serde::Serialize;

use rand::Rng;
use serde::Serializer;

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
    arkose_token: Option<ArkoseToken>,
}

impl<'a> TryFrom<PostNextConvoRequest<'a>> for PostConvoRequest<'a> {
    type Error = anyhow::Error;

    fn try_from(value: PostNextConvoRequest<'a>) -> Result<Self, Self::Error> {
        let ak = match GPT4Model::try_from(value.model) {
            Ok(_) => Some(ArkoseToken),
            Err(_) => None,
        };
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
            .arkose_token(ak)
            .build()?;
        Ok(body)
    }
}

impl<'a> TryFrom<PostContinueConvoRequest<'a>> for PostConvoRequest<'a> {
    type Error = anyhow::Error;

    fn try_from(value: PostContinueConvoRequest<'a>) -> Result<Self, Self::Error> {
        let ak = match GPT4Model::try_from(value.model) {
            Ok(_) => Some(ArkoseToken),
            Err(_) => None,
        };
        let body = PostConvoRequestBuilder::default()
            .action(Action::Continue)
            .conversation_id(value.conversation_id)
            .parent_message_id(value.parent_message_id)
            .model(value.model)
            .arkose_token(ak)
            .build()?;

        Ok(body)
    }
}

impl<'a> TryFrom<PostVaraintConvoRequest<'a>> for PostConvoRequest<'a> {
    type Error = anyhow::Error;

    fn try_from(value: PostVaraintConvoRequest<'a>) -> Result<Self, Self::Error> {
        let ak = match GPT4Model::try_from(value.model) {
            Ok(_) => Some(ArkoseToken),
            Err(_) => None,
        };
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
            .arkose_token(ak)
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
}

#[derive(Serialize, Builder)]
pub struct PostContinueConvoRequest<'a> {
    /// The conversation uses a model that usually remains the same throughout the conversation
    model: &'a str,
    /// Parent message ID, the message ID of the last ChatGPT reply.
    parent_message_id: &'a str,
    /// ID of a session. conversation_id Session ID.
    conversation_id: &'a str,
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
}

#[derive(PartialEq, Eq)]
pub enum GPT4Model {
    Gpt4model,
    Gpt4browsingModel,
    Gpt4pluginsModel,
}

impl TryFrom<&str> for GPT4Model {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "gpt-4" => Ok(GPT4Model::Gpt4model),
            "gpt-4-browsing" => Ok(GPT4Model::Gpt4browsingModel),
            "gpt-4-plugins" => Ok(GPT4Model::Gpt4pluginsModel),
            _ => Err(()),
        }
    }
}

#[derive(Clone)]
pub struct ArkoseToken;

impl Serialize for ArkoseToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let random_number = || -> u32 {
            let mut rng = rand::thread_rng();
            rng.gen_range(1..=100) + 1
        };
        let random_string = |length: usize| -> String {
            let charset: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
            let mut rng = rand::thread_rng();

            let result: String = (0..length)
                .map(|_| {
                    let random_index = rng.gen_range(0..charset.len());
                    charset[random_index] as char
                })
                .collect();

            result
        };
        serializer.serialize_str( &format!("{}.{}|r=us-east-1|meta=3|meta_width=300|metabgclr=transparent|metaiconclr=%%23555555|guitextcolor=%%23000000|pk={}|at=40|rid={}|ag=101|cdn_url=https%%3A%%2F%%2Ftcr9i.chat.openai.com%%2Fcdn%%2Ffc|lurl=https%%3A%%2F%%2Faudio-us-east-1.arkoselabs.com|surl=https%%3A%%2F%%2Ftcr9i.chat.openai.com|smurl=https%%3A%%2F%%2Ftcr9i.chat.openai.com%%2Fcdn%%2Ffc%%2Fassets%%2Fstyle-manager",
        random_string(7), random_string(10), "35536E1E-65B4-4D96-9D97-6ADB7EFF8147", random_number()))
    }
}
