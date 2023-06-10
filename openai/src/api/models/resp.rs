use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::api::Success;

use super::Author;

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountPlan {
    pub is_paid_subscription_active: bool,
    pub subscription_plan: String,
    pub account_user_role: String,
    pub was_paid_customer: bool,
    pub has_customer_object: bool,
    pub subscription_expires_at_timestamp: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct ModelsCategories {
    pub category: String,
    pub human_category_name: String,
    pub subscription_level: String,
    pub default_model: String,
    pub browsing_model: Option<String>,
    pub code_interpreter_model: Option<String>,
    pub plugins_model: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct GetModelsResponse {
    pub models: Vec<Models>,
    pub categories: Vec<ModelsCategories>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Mapping {
    id: String,
    parent: Option<String>,
    message: Option<Message>,
    children: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    pub id: String,
    pub author: Author,
    pub create_time: f64,
    pub update_time: Option<f64>,
    pub status: String,
    pub content: Content,
    pub metadata: Metadata,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Content {
    pub content_type: String,
    pub parts: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Metadata {
    message_type: Option<String>,
    model_slug: Option<String>,
    #[serde(rename = "timestamp_")]
    timestamp: Option<String>,
    finish_details: Option<FinishDetails>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FinishDetails {
    #[serde(rename = "type")]
    pub _type: Option<String>,
    pub stop: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConversationItems {
    pub id: String,
    pub title: String,
    pub create_time: String,
    pub update_time: String,
    pub current_node: Option<String>,
    pub mapping: Option<HashMap<String, Mapping>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetConversationsResponse {
    pub items: Vec<ConversationItems>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_missing_conversations: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetConversationResonse {
    pub title: String,
    pub create_time: f64,
    pub update_time: f64,
    pub mapping: HashMap<String, Mapping>,
    pub current_node: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PostConversationContent {
    pub content_type: String,
    pub parts: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PostConversationMessage {
    pub id: String,
    pub author: Author,
    pub create_time: f64,
    pub update_time: String,
    pub content: Content,
    pub status: String,
    pub end_turn: bool,
    pub weight: i64,
    pub metadata: PostConversationMetadata,
    pub recipient: String,
}

#[derive(Serialize, Deserialize)]
pub struct PostConversationMetadata {
    pub message_type: String,
    pub model_slug: String,
    pub finish_details: FinishDetails,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PostConversationResponse {
    pub message: Message,
    pub conversation_id: String,
    pub error: Option<String>,
}

impl PostConversationResponse {
    pub fn create_time(&self) -> i64 {
        self.message.create_time as i64
    }

    pub fn author(&self) -> String {
        self.message.author.role.to_string()
    }

    pub fn message_type(&self) -> &str {
        self.message.content.content_type.as_ref()
    }

    pub fn message(self) -> Vec<String> {
        self.message.content.parts
    }

    pub fn conversation_id(&self) -> &str {
        self.conversation_id.as_ref()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PatchConversationResponse {
    #[serde(default)]
    pub success: bool,
}

impl Success for PatchConversationResponse {
    fn ok(&self) -> bool {
        self.success
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageFeedbackResponse {
    pub id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub rating: String,
    pub create_time: String,
    pub embedded_conversation: String,
    pub storage_protocol: String,
}
