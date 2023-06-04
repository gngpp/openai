pub mod chat;
pub mod ios;
pub mod models;
pub mod service;

use async_trait::async_trait;

use self::models::{req, resp};

pub type ApiResult<T, E = anyhow::Error> = anyhow::Result<T, E>;

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
    #[error(" invalid cookie")]
    InvalidCookie,
    #[error("failed token deserialize")]
    DeserializeError,
    #[error("system time exception")]
    SystemTimeExceptionError,
    #[error("failed authentication")]
    FailedAuthenticationError,
    #[error("failed request")]
    FailedRequest,
    #[error("server error")]
    ServerError,
}

#[async_trait]
pub trait Api: Sync + Send {
    async fn get_models(&self) -> ApiResult<resp::ModelsResponse>;

    async fn account_check(&self) -> ApiResult<resp::AccountsCheckResponse>;

    async fn get_conversation(
        &self,
        conversation_id: &str,
    ) -> ApiResult<resp::GetConversationResonse>;

    async fn get_conversations(&self) -> ApiResult<resp::GetConversationsResponse>;

    async fn create_conversation(&self, payload: req::CreateConversationRequest) -> ApiResult<()>;

    async fn delete_conversation(
        &self,
        payload: req::DeleteConversationRequest,
    ) -> ApiResult<resp::DeleteConversationResponse>;

    async fn delete_conversations(&self, payload: req::DeleteConversationRequest) -> ApiResult<()>;

    async fn rename_conversation(
        &self,
        payload: req::RenameConversationRequest,
    ) -> ApiResult<resp::RenameConversationResponse>;
}

trait RefreshToken: Sync + Send {
    /// refresh access token
    fn refresh_token(&mut self, access_token: String);
}

trait ToConversationID {
    /// conversation to url subpath
    fn to_conversation_id(&self) -> &str;
}
