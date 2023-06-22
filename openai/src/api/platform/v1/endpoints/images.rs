use crate::api::platform::v1::api::file_from_disk_to_form_part;
use crate::api::platform::v1::api::Client;
use crate::api::platform::v1::error::APIError;
use crate::api::platform::v1::resources::image::{
    CreateImageParameters, CreateImageVariationParameters, EditImageParameters, ImageResponse,
};
use serde_json::Value;

pub struct Images<'a> {
    pub client: &'a Client,
}

impl Client {
    pub fn images(&self) -> Images {
        Images { client: self }
    }
}

impl Images<'_> {
    pub async fn create(
        &self,
        parameters: CreateImageParameters,
    ) -> Result<ImageResponse, APIError> {
        let response = self.client.post("/images/generations", &parameters).await?;

        let value: Value = serde_json::from_str(&response).unwrap();
        let create_image_response: ImageResponse = serde_json::from_value(value)
            .map_err(|error| APIError::ParseError(error.to_string()))?;

        Ok(create_image_response)
    }

    pub async fn edit(&self, parameters: EditImageParameters) -> Result<ImageResponse, APIError> {
        let mut form = reqwest::multipart::Form::new();

        let image = file_from_disk_to_form_part(parameters.image).await?;
        form = form.part("image", image);

        form = form.text("prompt", parameters.prompt);

        if let Some(mask) = parameters.mask {
            let image = file_from_disk_to_form_part(mask).await?;
            form = form.part("mask", image);
        }

        if let Some(n) = parameters.n {
            form = form.text("n", n.to_string());
        }

        if let Some(size) = parameters.size {
            form = form.text("size", size.to_string());
        }

        if let Some(response_format) = parameters.response_format {
            form = form.text("response_format", response_format.to_string());
        }

        let response = self.client.post_with_form("/images/edits", form).await?;

        let value: Value = serde_json::from_str(&response).unwrap();
        let create_image_response: ImageResponse = serde_json::from_value(value)
            .map_err(|error| APIError::ParseError(error.to_string()))?;

        Ok(create_image_response)
    }

    pub async fn variation(
        &self,
        parameters: CreateImageVariationParameters,
    ) -> Result<ImageResponse, APIError> {
        let mut form = reqwest::multipart::Form::new();

        let image = file_from_disk_to_form_part(parameters.image).await?;
        form = form.part("image", image);

        if let Some(n) = parameters.n {
            form = form.text("n", n.to_string());
        }

        if let Some(size) = parameters.size {
            form = form.text("size", size.to_string());
        }

        if let Some(response_format) = parameters.response_format {
            form = form.text("response_format", response_format.to_string());
        }

        let response = self
            .client
            .post_with_form("/images/variations", form)
            .await?;

        let value: Value = serde_json::from_str(&response).unwrap();
        let create_image_response: ImageResponse = serde_json::from_value(value)
            .map_err(|error| APIError::ParseError(error.to_string()))?;

        Ok(create_image_response)
    }
}
