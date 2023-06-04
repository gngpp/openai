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

#[derive(Serialize, Deserialize)]
pub struct Mapping {
    id: String,
    parent: Option<String>,
    message: Option<Message>,
    children: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    id: String,
    create_time: f64,
    status: String,
}

#[derive(Serialize, Deserialize)]
pub struct Content {
    pub content_type: String,
    pub parts: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ConversationItems {
    pub id: String,
    pub title: String,
    pub create_time: String,
    pub update_time: String,
    pub current_node: Option<String>,
    pub mapping: Option<HashMap<String, Mapping>>,
}

#[derive(Serialize, Deserialize)]
pub struct GetConversationsResponse {
    pub items: Vec<ConversationItems>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
    pub has_missing_conversations: bool,
}

#[derive(Serialize, Deserialize)]
pub struct GetConversationResonse {}

#[derive(Serialize, Deserialize)]
pub struct DeleteConversationResponse {
    pub success: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RenameConversationResponse {
    pub success: bool,
}
