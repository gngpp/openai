use crate::api::platform::v1::api::Client;
use crate::api::platform::v1::error::APIError;
use crate::api::platform::v1::resources::model::Model;
use serde_json::Value;

pub struct Models<'a> {
    pub client: &'a Client,
}

impl Client {
    pub fn models(&self) -> Models {
        Models { client: self }
    }
}

impl Models<'_> {
    pub async fn list(&self) -> Result<Vec<Model>, APIError> {
        let response = self.client.get("/models").await?;

        let value: Value = serde_json::from_str(&response).unwrap();
        let models: Vec<Model> = serde_json::from_value(value["data"].clone())
            .map_err(|error| APIError::ParseError(error.to_string()))?;

        Ok(models)
    }

    pub async fn get(&self, model_id: &str) -> Result<Model, APIError> {
        let path = format!("/models/{}", model_id);

        let response = self.client.get(&path).await?;

        let value: Value = serde_json::from_str(&response).unwrap();
        let model_response: Model = serde_json::from_value(value)
            .map_err(|error| APIError::ParseError(error.to_string()))?;

        Ok(model_response)
    }
}
