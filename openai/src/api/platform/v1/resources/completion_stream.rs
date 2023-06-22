use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompletionStreamResponse {
    pub id: String,
    pub object: String,
    pub created: u32,
    pub model: String,
    pub choices: Vec<CompletionStreamChoice>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CompletionStreamChoice {
    pub text: String,
    pub index: u32,
}
