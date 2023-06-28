use crate::platform::v1::resources::file::File;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateFineTuneParameters {
    pub training_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n_epochs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub learning_rate_multiplier: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_loss_weight: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compute_classification_metrics: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification_n_classes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification_positive_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classification_betas: Option<Vec<f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FineTune {
    pub id: String,
    pub model: String,
    pub created_at: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<FineTuneEvent>>,
    pub fine_tuned_model: Option<String>,
    pub hyperparams: Hyperparams,
    pub organization_id: String,
    pub result_files: Vec<File>,
    pub status: String,
    pub validation_files: Vec<File>,
    pub training_files: Vec<File>,
    pub updated_at: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FineTuneEvent {
    pub object: String,
    pub created_at: u32,
    pub level: String,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hyperparams {
    pub batch_size: Option<u32>,
    pub learning_rate_multiplier: Option<f32>,
    pub n_epochs: u32,
    pub prompt_loss_weight: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeletedFineTuneModel {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
