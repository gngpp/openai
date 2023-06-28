use crate::platform::v1::api::Client;
use crate::platform::v1::error::APIError;
use crate::platform::v1::resources::completion::{CompletionParameters, CompletionResponse};
use serde_json::Value;

#[cfg(feature = "stream")]
use crate::platform::v1::resources::completion_stream::CompletionStreamResponse;
#[cfg(feature = "stream")]
use crate::platform::v1::resources::shared::StopToken;
#[cfg(feature = "stream")]
use futures::Stream;
#[cfg(feature = "stream")]
use serde::Serialize;
#[cfg(feature = "stream")]
use std::collections::HashMap;
#[cfg(feature = "stream")]
use std::pin::Pin;

#[cfg(feature = "simple")]
use crate::platform::v1::resources::completion::SimpleCompletionParameters;

pub struct Completions<'a> {
    pub client: &'a Client,
}

impl Client {
    pub fn completions(&self) -> Completions {
        Completions { client: self }
    }
}

impl Completions<'_> {
    pub async fn create(
        &self,
        parameters: CompletionParameters,
    ) -> Result<CompletionResponse, APIError> {
        let response = self.client.post("/completions", &parameters).await?;

        let value: Value = serde_json::from_str(&response).unwrap();
        let completion_response: CompletionResponse = serde_json::from_value(value)
            .map_err(|error| APIError::ParseError(error.to_string()))?;

        Ok(completion_response)
    }

    #[deprecated(since = "0.2.8")]
    #[cfg(feature = "simple")]
    pub async fn create_simple(
        &self,
        parameters: SimpleCompletionParameters,
    ) -> Result<CompletionResponse, APIError> {
        let response = self.client.post("/completions", &parameters).await?;

        let value: Value = serde_json::from_str(&response).unwrap();
        let completion_response: CompletionResponse = serde_json::from_value(value)
            .map_err(|error| APIError::ParseError(error.to_string()))?;

        Ok(completion_response)
    }

    #[cfg(feature = "stream")]
    pub async fn create_stream(
        &self,
        parameters: CompletionParameters,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<CompletionStreamResponse, APIError>> + Send>>,
        APIError,
    > {
        let stream_parameters = CompletionStreamParameters {
            model: parameters.model,
            prompt: parameters.prompt,
            suffix: None,
            max_tokens: Some(50),
            temperature: parameters.temperature,
            top_p: parameters.top_p,
            n: parameters.n,
            stream: true,
            logprobs: parameters.logprobs,
            echo: parameters.echo,
            stop: parameters.stop,
            presence_penalty: parameters.presence_penalty,
            frequency_penalty: parameters.frequency_penalty,
            best_of: parameters.best_of,
            logit_bias: parameters.logit_bias,
        };

        Ok(self
            .client
            .post_stream("/completions", &stream_parameters)
            .await)
    }
}

#[cfg(feature = "stream")]
#[derive(Serialize, Debug)]
struct CompletionStreamParameters {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suffix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub echo: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<StopToken>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_of: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<HashMap<String, serde_json::Value>>,
}
