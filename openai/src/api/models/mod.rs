use derive_builder::Builder;
use rand::Rng;
use serde::Serializer;
use serde::{Deserialize, Serialize};
pub mod req;
pub mod resp;

/// A role of a message sender, can be:
/// - `System`, for starting system message, that sets the tone of model
/// - `Assistant`, for messages sent by ChatGPT
/// - `User`, for messages sent by user
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize, Eq, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// A system message, automatically sent at the start to set the tone of the model
    System,
    /// A message sent by ChatGPT
    Assistant,
    /// A message sent by the user
    User,
}

impl ToString for Role {
    fn to_string(&self) -> String {
        match self {
            Role::System => "system".to_string(),
            Role::Assistant => "assistant".to_string(),
            Role::User => "user".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Builder, Clone, Debug)]
pub struct Author {
    pub role: Role,
}

#[derive(PartialEq, Eq)]
pub enum GPT4Model {
    Gpt4model,
    Gpt4browsingModel,
    Gpt4pluginsModel,
}

impl TryFrom<&str> for GPT4Model {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "gpt-4" => Ok(GPT4Model::Gpt4model),
            "gpt-4-browsing" => Ok(GPT4Model::Gpt4browsingModel),
            "gpt-4-plugins" => Ok(GPT4Model::Gpt4pluginsModel),
            _ => Err(()),
        }
    }
}

#[derive(Clone)]
pub struct ArkoseToken;

impl Serialize for ArkoseToken {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let random_number = || -> u32 {
            let mut rng = rand::thread_rng();
            rng.gen_range(1..=100) + 1
        };
        let random_string = |length: usize| -> String {
            let charset: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
            let mut rng = rand::thread_rng();

            let result: String = (0..length)
                .map(|_| {
                    let random_index = rng.gen_range(0..charset.len());
                    charset[random_index] as char
                })
                .collect();

            result
        };
        serializer.serialize_str( &format!("{}.{}|r=us-east-1|meta=3|meta_width=300|metabgclr=transparent|metaiconclr=%%23555555|guitextcolor=%%23000000|pk={}|at=40|rid={}|ag=101|cdn_url=https%%3A%%2F%%2Ftcr9i.chat.openai.com%%2Fcdn%%2Ffc|lurl=https%%3A%%2F%%2Faudio-us-east-1.arkoselabs.com|surl=https%%3A%%2F%%2Ftcr9i.chat.openai.com|smurl=https%%3A%%2F%%2Ftcr9i.chat.openai.com%%2Fcdn%%2Ffc%%2Fassets%%2Fstyle-manager",
        random_string(7), random_string(10), "35536E1E-65B4-4D96-9D97-6ADB7EFF8147", random_number()))
    }
}
