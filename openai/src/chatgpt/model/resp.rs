use std::collections::HashMap;

use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::chatgpt::Success;

use super::{Author, Role};

#[derive(Deserialize, Debug)]
pub struct AccountPlan {
    pub is_paid_subscription_active: bool,
    pub subscription_plan: String,
    pub account_user_role: String,
    pub was_paid_customer: bool,
    pub has_customer_object: bool,
    pub subscription_expires_at_timestamp: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct GetAccountsCheckResponse {
    pub account_plan: AccountPlan,
    pub user_country: String,
    pub features: Vec<String>,
}

impl Success for GetAccountsCheckResponse {
    fn ok(&self) -> bool {
        !self.user_country.is_empty()
    }
}

#[derive(Deserialize, Debug)]
pub struct ModelsCategories {
    pub category: String,
    pub human_category_name: String,
    pub subscription_level: String,
    pub default_model: String,
    pub browsing_model: Option<String>,
    pub code_interpreter_model: Option<String>,
    pub plugins_model: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct Models {
    pub slug: String,
    pub max_tokens: i64,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
}

impl Models {
    pub fn model_name(&self) -> &str {
        self.slug.as_ref()
    }

    pub fn description(&self) -> &str {
        self.description.as_ref()
    }

    pub fn max_tokens(&self) -> i64 {
        self.max_tokens
    }
}

#[derive(Deserialize, Debug)]
pub struct GetModelsResponse {
    pub models: Vec<Models>,
    pub categories: Vec<ModelsCategories>,
}

impl GetModelsResponse {
    pub fn real_models(&self) -> Vec<&str> {
        self.models.iter().map(|v| v.slug.as_str()).collect()
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Mapping {
    pub id: String,
    pub parent: Option<String>,
    pub message: Option<Message>,
    pub children: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct Message {
    pub id: String,
    pub author: Author,
    pub create_time: f64,
    pub update_time: Option<f64>,
    pub status: String,
    pub content: Content,
    pub metadata: Metadata,
    pub end_turn: Option<bool>,
}

#[derive(Deserialize, Debug)]
pub struct Content {
    pub content_type: String,
    pub parts: Vec<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Metadata {
    message_type: Option<String>,
    model_slug: Option<String>,
    #[serde(rename = "timestamp_")]
    timestamp: Option<String>,
    finish_details: Option<FinishDetails>,
}

#[derive(Deserialize, Debug)]
pub struct FinishDetails {
    #[serde(rename = "type")]
    pub _type: Option<String>,
    pub stop: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ConvoItems {
    pub id: String,
    pub title: String,
    pub create_time: String,
    pub update_time: String,
    pub current_node: Option<String>,
    pub mapping: Option<HashMap<String, Mapping>>,
}

#[derive(Deserialize, Debug)]
pub struct GetConvosResponse {
    pub items: Vec<ConvoItems>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_missing_conversations: bool,
}

#[derive(Deserialize, Debug)]
pub struct GetConvoResonse {
    pub title: String,
    pub create_time: f64,
    pub update_time: f64,
    pub mapping: HashMap<String, Mapping>,
    pub current_node: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct PostConvoContent {
    pub content_type: String,
    pub parts: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct PostConvoMessage {
    pub id: String,
    pub author: Author,
    pub create_time: f64,
    pub update_time: String,
    pub content: Content,
    pub status: String,
    pub end_turn: bool,
    pub weight: i64,
    pub metadata: PostConvoMetadata,
    pub recipient: String,
}

#[derive(Deserialize, Debug)]
pub struct PostConvoMetadata {
    pub message_type: String,
    pub model_slug: String,
    pub finish_details: FinishDetails,
}

#[derive(Deserialize, Debug)]
pub struct ConvoResponse {
    pub message: Message,
    pub conversation_id: String,
    pub error: Option<String>,
}

impl ConvoResponse {
    pub fn end_turn(&self) -> Option<bool> {
        self.message.end_turn
    }

    pub fn create_time(&self) -> i64 {
        self.message.create_time as i64
    }

    pub fn role(&self) -> &Role {
        &self.message.author.role
    }

    pub fn message_type(&self) -> &str {
        &self.message.content.content_type
    }

    pub fn message(self) -> Vec<String> {
        self.message
            .content
            .parts
            .into_iter()
            .map(|c| {
                if let Some(c) = crate::unescape::unescape(&c) {
                    return c;
                }
                c
            })
            .collect()
    }

    pub fn message_id(&self) -> &str {
        &self.message.id
    }

    pub fn conversation_id(&self) -> &str {
        &self.conversation_id
    }
}

#[derive(Deserialize, Debug)]
pub struct ModerationResponse {
    pub conversation_id: String,
    pub message_id: String,
    pub is_completion: bool,
}

#[derive(Debug)]
pub enum PostConvoResponse {
    Conversation(ConvoResponse),
    Moderation(ModerationResponse),
}

impl<'de> Deserialize<'de> for PostConvoResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        if let Some(map) = value.as_object() {
            if map.contains_key("is_completion")
                || map.contains_key("message_id")
                || map.contains_key("moderation_response")
            {
                if let Ok(ok) = serde_json::from_value::<ModerationResponse>(value) {
                    return Ok(PostConvoResponse::Moderation(ok));
                }
            } else {
                if let Ok(ok) = serde_json::from_value::<ConvoResponse>(value) {
                    return Ok(PostConvoResponse::Conversation(ok));
                }
            }
        }
        Err(serde::de::Error::custom("deserialization body failed"))
    }
}

#[derive(Deserialize, Debug)]
pub struct PatchConvoResponse {
    #[serde(default)]
    pub success: bool,
}

impl Success for PatchConvoResponse {
    fn ok(&self) -> bool {
        self.success
    }
}

#[derive(Deserialize, Debug)]
pub struct PostConvoGenTitleResponse {
    pub title: Option<String>,
    pub message: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct MessageFeedbackResponse {
    pub id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub rating: String,
    pub create_time: String,
    pub embedded_conversation: String,
    pub storage_protocol: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAccountsCheckV4Response {
    pub accounts: Accounts,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Accounts {
    pub default: Default,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Default {
    pub account: Account,
    pub features: Vec<String>,
    pub entitlement: Entitlement,
    #[serde(rename = "last_active_subscription")]
    pub last_active_subscription: LastActiveSubscription,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    #[serde(rename = "account_user_role")]
    pub account_user_role: String,
    #[serde(rename = "account_user_id")]
    pub account_user_id: String,
    pub processor: Processor,
    #[serde(rename = "account_id")]
    pub account_id: String,
    #[serde(rename = "is_most_recent_expired_subscription_gratis")]
    pub is_most_recent_expired_subscription_gratis: bool,
    #[serde(rename = "has_previously_paid_subscription")]
    pub has_previously_paid_subscription: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Processor {
    pub a001: A001,
    pub b001: B001,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct A001 {
    #[serde(rename = "has_customer_object")]
    pub has_customer_object: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct B001 {
    #[serde(rename = "has_transaction_history")]
    pub has_transaction_history: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entitlement {
    #[serde(rename = "subscription_id")]
    pub subscription_id: Value,
    #[serde(rename = "has_active_subscription")]
    pub has_active_subscription: bool,
    #[serde(rename = "subscription_plan")]
    pub subscription_plan: String,
    #[serde(rename = "expires_at")]
    pub expires_at: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastActiveSubscription {
    #[serde(rename = "subscription_id")]
    pub subscription_id: Value,
    #[serde(rename = "purchase_origin_platform")]
    pub purchase_origin_platform: String,
    #[serde(rename = "will_renew")]
    pub will_renew: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetConvoLimitResponse {
    #[serde(rename = "message_cap")]
    pub message_cap: i64,
    #[serde(rename = "message_cap_window")]
    pub message_cap_window: i64,
    #[serde(rename = "message_disclaimer")]
    pub message_disclaimer: MessageDisclaimer,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageDisclaimer {
    pub textarea: String,
    #[serde(rename = "model-switcher")]
    pub model_switcher: String,
}
