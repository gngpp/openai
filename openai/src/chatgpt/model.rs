use serde::{Deserialize, Serialize};

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
pub struct ModelsData {
    pub models: Vec<Models>,
    pub categories: Vec<ModelsCategories>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServerStatus {
    status: Status,
    #[serde(default)]
    message: String,
    #[serde(default)]
    button_url: String,
    #[serde(default)]
    button_title: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Status {
    Normal,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TitleData {
    #[serde(default)]
    pub title: String,
}
