pub mod chatgpt;
pub mod models;
pub mod opengpt;
pub mod service;

use async_trait::async_trait;
use fficall::StreamLine;

use self::models::{req, resp};

pub type ApiResult<T, E = ApiError> = anyhow::Result<T, E>;

pub enum Method {
    GET,
    POST,
    PATCH,
    PUT,
    DELETE,
}

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("failed to cookie")]
    FailedGetCookie,
    #[error("invalid cookie")]
    InvalidCookie,
    #[error(transparent)]
    SerdeDeserializeError(#[from] serde_json::error::Error),
    #[error(transparent)]
    ReqwestJsonDeserializeError(#[from] reqwest::Error),
    #[error(transparent)]
    AnyhowJsonDeserializeError(#[from] anyhow::Error),
    #[error("failed serialize `{0}`")]
    SerializeError(String),
    #[error("system time exception")]
    SystemTimeExceptionError,
    #[error("failed authentication `{0}`")]
    BadAuthenticationError(String),
    #[error("failed request `{0}`")]
    FailedRequest(String),
    #[error("redirection error")]
    RedirectionError,
    #[error("bad request `{0}`")]
    BadRequest(String),
    #[error("server error")]
    ServerError,
}

#[async_trait]
pub trait Api: Sync + Send {
    async fn get_models(&self) -> ApiResult<resp::ModelsResponse>;

    async fn account_check(&self) -> ApiResult<resp::AccountsCheckResponse>;

    async fn get_conversation(
        &self,
        req: req::GetConversationRequest,
    ) -> ApiResult<resp::GetConversationResonse>;

    async fn get_conversations(
        &self,
        req: req::GetConversationRequest,
    ) -> ApiResult<resp::GetConversationsResponse>;

    async fn create_conversation(
        &self,
        req: req::CreateConversationRequest,
    ) -> ApiResult<Box<dyn StreamLine<resp::CreateConversationResponse>>>;

    async fn clear_conversation(
        &self,
        req: req::ClearConversationRequest,
    ) -> ApiResult<resp::ClearConversationResponse>;

    async fn clear_conversations(
        &self,
        req: req::ClearConversationRequest,
    ) -> ApiResult<resp::ClearConversationResponse>;

    async fn rename_conversation(
        &self,
        req: req::RenameConversationRequest,
    ) -> ApiResult<resp::RenameConversationResponse>;

    async fn message_feedback(
        &self,
        req: req::MessageFeedbackRequest,
    ) -> ApiResult<resp::MessageFeedbackResponse>;
}

trait RefreshToken: Sync + Send {
    /// refresh access token
    fn refresh_token(&mut self, access_token: String);
}

pub struct StreamResponseWrapper(fficall::model::ResponsePayload);

impl<T: serde::de::DeserializeOwned> StreamLine<T> for StreamResponseWrapper {
    fn next(&self) -> fficall::FiiCallResult<Option<T>> {
        if let Some(body) = self.0.next()? {
            return Ok(Some(
                serde_json::from_str::<T>(&body).map_err(ApiError::SerdeDeserializeError)?,
            ));
        }
        Ok(None)
    }

    fn stop(self) -> fficall::FiiCallResult<()> {
        self.0.stop()
    }
}
