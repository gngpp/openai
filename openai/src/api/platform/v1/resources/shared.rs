use serde::{Deserialize, Serialize};

#[cfg(feature = "download")]
use rand::{distributions::Alphanumeric, Rng};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BaseModel {
    name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum FinishReason {
    #[serde(rename(deserialize = "stop"))]
    StopSequenceReached,
    #[serde(rename(deserialize = "length"))]
    TokenLimitReached,
    #[serde(rename(deserialize = "content_filter"))]
    ContentFilterFlagged,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum StopToken {
    String(String),
    Array(Vec<String>),
}

#[cfg(feature = "download")]
pub fn generate_file_name(path: &str, length: u32, file_type: &str) -> String {
    let random_file_name: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length as usize)
        .map(char::from)
        .collect();

    format!("{}/{}.{}", path, random_file_name, file_type)
}
