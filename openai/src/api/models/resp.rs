use std::collections::HashMap;

use serde::{Deserialize, Serialize};

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
pub struct AccountsCheckResponse {
    pub account_plan: AccountPlan,
    pub user_country: String,
    pub features: Vec<String>,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct ModelsResponse {
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
    id: String,
    author: Author,
    create_time: f64,
    status: String,
    content: Content,
    metadata: Metadata,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Content {
    pub content_type: String,
    pub parts: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Metadata {
    model_slug: Option<String>,
    #[serde(rename = "timestamp_")]
    timestamp: Option<String>,
    finish_details: Option<FinishDetails>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FinishDetails {
    #[serde(rename = "type")]
    _type: Option<String>,
    stop: Option<String>,
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
pub struct Author {
    role: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateConversationContent {
    content_type: String,
    parts: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateConversationFinishDetails {
    #[serde(rename = "type")]
    _type: String,
    stop: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateConversationMessage {
    pub id: String,
    pub author: Author,
    pub create_time: f64,
    pub update_time: String,
    pub content: Content,
    pub status: String,
    pub end_turn: bool,
    pub weight: i64,
    pub metadata: CreateConversationMetadata,
    pub recipient: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateConversationMetadata {
    pub message_type: String,
    pub model_slug: String,
    pub finish_details: CreateConversationFinishDetails,
}

#[derive(Serialize, Deserialize)]
pub struct CreateConversationResponse {
    pub message: Message,
    pub conversation_id: String,
    pub error: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClearConversationResponse {
    #[serde(default)]
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RenameConversationResponse {
    #[serde(default)]
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageFeedbackResponse {
    pub id: String,
    pub conversation_id: String,
    pub user_id: String,
    pub rating: String,
    pub create_time: String,
    //pub  workspace_id: Option<String>,
    //pub  content: String,
    pub embedded_conversation: String,
    pub storage_protocol: String,
}
