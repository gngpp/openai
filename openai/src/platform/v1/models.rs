use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum OpenAIModel {
    #[serde(rename = "gpt-4")]
    Gpt4,
    #[serde(rename = "gpt-4-0314")]
    Gpt4_0314,
    #[serde(rename = "gpt-4-32k")]
    Gpt4_32K,
    #[serde(rename = "gpt-4-32k-0314")]
    Gpt4_32K0314,
    #[serde(rename = "gpt-3.5-turbo")]
    Gpt3_5Turbo,
    #[serde(rename = "gpt-3.5-turbo-0301")]
    Gpt3_5Turbo0301,
    #[serde(rename = "text-davinci-003")]
    TextDavinci003,
    #[serde(rename = "text-davinci-edit-001")]
    TextDavinciEdit001,
    #[serde(rename = "text-curie-001")]
    TextCurie001,
    #[serde(rename = "text-babbage-001")]
    TextBabbage001,
    #[serde(rename = "text-ada-001")]
    TextAda001,
    #[serde(rename = "text-embedding-ada-002")]
    TextEmbeddingAda002,
    #[serde(rename = "whisper-1")]
    Whisper1,
    #[serde(rename = "text-moderation-stable")]
    TextModerationStable,
    #[serde(rename = "text-moderation-latest")]
    TextModerationLatest,
}

impl Display for OpenAIModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                OpenAIModel::Gpt4 => "gpt-4",
                OpenAIModel::Gpt4_0314 => "gpt-4-0314",
                OpenAIModel::Gpt4_32K => "gpt-4-32k",
                OpenAIModel::Gpt4_32K0314 => "gpt-4-32k-0314",
                OpenAIModel::Gpt3_5Turbo => "gpt-3.5-turbo-0301",
                OpenAIModel::Gpt3_5Turbo0301 => "gpt-3.5-turbo",
                OpenAIModel::TextDavinci003 => "text-davinci-003",
                OpenAIModel::TextDavinciEdit001 => "text-davinci-edit-001",
                OpenAIModel::TextCurie001 => "text-curie-001",
                OpenAIModel::TextBabbage001 => "text-babbage-001",
                OpenAIModel::TextAda001 => "text-ada-001",
                OpenAIModel::TextEmbeddingAda002 => "text-embedding-ada-002",
                OpenAIModel::Whisper1 => "whisper-1",
                OpenAIModel::TextModerationStable => "text-moderation-stable",
                OpenAIModel::TextModerationLatest => "text-moderation-latest",
            }
        )
    }
}
