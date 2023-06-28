use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct File {
    pub id: String,
    pub object: String,
    pub bytes: u32,
    pub created_at: u32,
    pub filename: String,
    pub purpose: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UploadFileParameters {
    pub file: String,
    pub purpose: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeletedFile {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
