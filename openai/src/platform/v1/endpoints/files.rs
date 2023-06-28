use crate::platform::v1::api::file_from_disk_to_form_part;
use crate::platform::v1::api::Client;
use crate::platform::v1::error::APIError;
use crate::platform::v1::resources::file::{DeletedFile, File, UploadFileParameters};
use serde_json::Value;

pub struct Files<'a> {
    pub client: &'a Client,
}

impl Client {
    pub fn files(&self) -> Files {
        Files { client: self }
    }
}

impl Files<'_> {
    pub async fn list(&self) -> Result<Vec<File>, APIError> {
        let response = self.client.get("/files").await?;

        let value: Value = serde_json::from_str(&response).unwrap();
        let files: Vec<File> = serde_json::from_value(value["data"].clone())
            .map_err(|error| APIError::ParseError(error.to_string()))?;

        Ok(files)
    }

    pub async fn upload(&self, parameters: UploadFileParameters) -> Result<File, APIError> {
        let mut form = reqwest::multipart::Form::new();

        let file = file_from_disk_to_form_part(parameters.file).await?;
        form = form.part("file", file);

        form = form.text("purpose", parameters.purpose);

        let response = self.client.post_with_form("/files", form).await?;

        let value: Value = serde_json::from_str(&response).unwrap();
        let file_response: File = serde_json::from_value(value)
            .map_err(|error| APIError::ParseError(error.to_string()))?;

        Ok(file_response)
    }

    pub async fn delete(&self, id: &str) -> Result<DeletedFile, APIError> {
        let response = self.client.delete(format!("/files/{id}").as_str()).await?;

        let value: Value = serde_json::from_str(&response).unwrap();
        let deleted_file_response: DeletedFile = serde_json::from_value(value)
            .map_err(|error| APIError::ParseError(error.to_string()))?;

        Ok(deleted_file_response)
    }

    pub async fn retrieve(&self, id: &str) -> Result<File, APIError> {
        let response = self.client.get(format!("/files/{id}").as_str()).await?;

        let value: Value = serde_json::from_str(&response).unwrap();
        let file_response: File = serde_json::from_value(value)
            .map_err(|error| APIError::ParseError(error.to_string()))?;

        Ok(file_response)
    }

    pub async fn retrieve_content(&self, id: &str) -> Result<String, APIError> {
        let response = self
            .client
            .get(format!("/files/{id}/content").as_str())
            .await?;

        Ok(response)
    }
}
