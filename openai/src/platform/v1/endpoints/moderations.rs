use crate::platform::v1::api::Client;
use crate::platform::v1::error::APIError;
use crate::platform::v1::resources::moderation::{ModerationParameters, ModerationResponse};
use serde_json::Value;

pub struct Moderations<'a> {
    pub client: &'a Client,
}

impl Client {
    pub fn moderations(&self) -> Moderations {
        Moderations { client: self }
    }
}

impl Moderations<'_> {
    pub async fn create(
        &self,
        parameters: ModerationParameters,
    ) -> Result<ModerationResponse, APIError> {
        let response = self.client.post("/moderations", &parameters).await?;

        let value: Value = serde_json::from_str(&response).unwrap();

        let moderation_response: ModerationResponse = serde_json::from_value(value)
            .map_err(|error| APIError::ParseError(error.to_string()))?;

        Ok(moderation_response)
    }
}
