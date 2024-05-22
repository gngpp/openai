use std::str::FromStr;

use serde::{Serialize, Serializer};

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum GPTModel {
    Gpt35,
    Gpt4,
    Gpt4Mobile,
    GptGizmo,
}

impl Serialize for GPTModel {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let model = match self {
            GPTModel::Gpt35 => "text-davinci-002-render-sha",
            GPTModel::Gpt4 => "gpt-4",
            GPTModel::Gpt4Mobile => "gpt-4-mobile",
            GPTModel::GptGizmo => "gpt-4-gizmo",
        };
        serializer.serialize_str(model)
    }
}

impl GPTModel {
    pub fn is_gpt3(&self) -> bool {
        match self {
            GPTModel::Gpt35 => true,
            _ => false,
        }
    }

    pub fn is_gpt4(&self) -> bool {
        match self {
            GPTModel::Gpt4 | GPTModel::Gpt4Mobile => true,
            _ => false,
        }
    }

    pub fn is_gizmo(&self) -> bool {
        match self {
            GPTModel::GptGizmo => true,
            _ => false,
        }
    }
}

impl FromStr for GPTModel {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            // If the model starts with gpt-3.5 or text-davinci/code-davinci, we assume it's gpt-3.5
            s if s.starts_with("gpt-3.5")
                || s.starts_with("text-davinci")
                || s.starts_with("code-davinci") =>
            {
                Ok(GPTModel::Gpt35)
            }
            s if s.starts_with("g-")=> {
                Ok(GPTModel::GptGizmo)
            }
            // If the model is gpt-4-mobile, we assume it's gpt-4-mobile
            "gpt-4-mobile" => Ok(GPTModel::Gpt4Mobile),
            // If the model starts with gpt-4, we assume it's gpt-4
            s if s.starts_with("gpt-4") => Ok(GPTModel::Gpt4),
            _ => anyhow::bail!("Invalid GPT model: {value}"),
        }
    }
}
