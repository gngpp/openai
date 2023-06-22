use crate::api::platform::v1::api::file_from_disk_to_form_part;
use crate::api::platform::v1::api::Client;
use crate::api::platform::v1::error::APIError;
use crate::api::platform::v1::resources::audio::{
    AudioTranscriptionParameters, AudioTranslationParameters,
};

pub struct Audio<'a> {
    pub client: &'a Client,
}

impl Client {
    pub fn audio(&self) -> Audio {
        Audio { client: self }
    }
}

impl Audio<'_> {
    pub async fn create_transcription(
        &self,
        parameters: AudioTranscriptionParameters,
    ) -> Result<String, APIError> {
        let mut form = reqwest::multipart::Form::new();

        let file = file_from_disk_to_form_part(parameters.file).await?;
        form = form.part("file", file);

        form = form.text("model", parameters.model);

        if let Some(prompt) = parameters.prompt {
            form = form.text("prompt", prompt);
        }

        if let Some(response_format) = parameters.response_format {
            form = form.text("response_format", response_format.to_string());
        }

        if let Some(temperature) = parameters.temperature {
            form = form.text("temperature", temperature.to_string());
        }

        if let Some(language) = parameters.language {
            form = form.text("language", language.to_string());
        }

        let response = self
            .client
            .post_with_form("/audio/transcriptions", form)
            .await?;

        Ok(response)
    }

    pub async fn create_translation(
        &self,
        parameters: AudioTranslationParameters,
    ) -> Result<String, APIError> {
        let mut form = reqwest::multipart::Form::new();

        let file = file_from_disk_to_form_part(parameters.file).await?;
        form = form.part("file", file);

        form = form.text("model", parameters.model);

        if let Some(prompt) = parameters.prompt {
            form = form.text("prompt", prompt);
        }

        if let Some(response_format) = parameters.response_format {
            form = form.text("response_format", response_format.to_string());
        }

        if let Some(temperature) = parameters.temperature {
            form = form.text("temperature", temperature.to_string());
        }

        let response = self
            .client
            .post_with_form("/audio/translations", form)
            .await?;

        Ok(response)
    }
}
