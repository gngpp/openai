use crate::api::platform::v1::models::OpenAIModel;
use crate::api::platform::v1::resources::shared::StopToken;
use crate::api::platform::v1::resources::shared::{FinishReason, Usage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;

#[deprecated(since = "0.2.8")]
#[cfg(feature = "simple")]
#[derive(Serialize, Debug, Clone)]
pub struct SimpleChatCompletionParameters {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: u32,
}

#[derive(Serialize, Debug, Clone)]
pub struct ChatCompletionParameters {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<StopToken>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

impl Default for ChatCompletionParameters {
    fn default() -> Self {
        ChatCompletionParameters {
            model: OpenAIModel::Gpt3_5Turbo.to_string(),
            messages: vec![ChatMessage {
                role: Role::User,
                content: "Hello!".to_string(),
                name: None,
            }],
            temperature: None,
            top_p: None,
            n: None,
            stop: None,
            max_tokens: None,
            presence_penalty: None,
            frequency_penalty: None,
            logit_bias: None,
            user: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u32,
    pub model: String,
    pub choices: Vec<ChatCompletionChoice>,
    pub usage: Usage,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatCompletionChoice {
    pub index: u32,
    pub message: ChatMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<FinishReason>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Role::System => "System",
                Role::User => "User",
                Role::Assistant => "Assistant",
            }
        )
    }
}
