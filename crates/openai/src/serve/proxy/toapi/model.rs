use serde::Deserialize;

use crate::chatgpt::model::Role;
use serde::Serialize;
use typed_builder::TypedBuilder;

#[derive(Deserialize)]
pub struct Req {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Serialize, TypedBuilder, Clone)]
pub struct Resp<'a> {
    id: &'a str,
    object: &'a str,
    created: &'a i64,
    model: &'a str,
    choices: Vec<Choice<'a>>,
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<Usage>,
}

#[derive(Serialize, Deserialize, TypedBuilder, Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

#[derive(Serialize, TypedBuilder, Clone)]
pub struct Usage {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Serialize, TypedBuilder, Clone)]
pub struct Choice<'a> {
    pub index: i64,
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<Delta<'a>>,
    #[builder(default)]
    pub finish_reason: Option<&'a str>,
}

#[derive(Serialize, TypedBuilder, Clone)]
pub struct Delta<'a> {
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<&'a Role>,
    #[builder(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<&'a str>,
}



#[derive(Deserialize, Default , Clone)]
pub struct WSStreamData {
    pub data: Option<WSStreamDataBody>,
    #[serde(rename = "type")]
    pub msg_type: String,
}

#[derive(Deserialize, Default , Clone)]
pub struct WSStreamDataBody {
    pub body: String,
    pub conversation_id: String,
    pub more_body: bool,
}
