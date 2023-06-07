use std::{collections::HashMap, time::Duration};

use crate::api::ApiError;

use super::{
    models::{req, resp},
    Api, ApiResult, RefreshToken, StreamResponseWrapper,
};
use async_trait::async_trait;
use fficall::{
    model::{RequestMethod, RequestPayloadBuilder, ResponsePayload, StatusCode},
    StreamLine,
};
use serde::de::{self, DeserializeOwned};
use tokio::sync::RwLock;
use url::Url;

const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36";
const URL_CHATGPT_BASE: &str = "https://chat.openai.com";

pub struct OpenGPT {
    access_token: RwLock<String>,
    req_headers: HashMap<String, String>,
    proxy: Option<Url>,
    timeout: Duration,
    cookie_store: bool,
}

impl OpenGPT {
    async fn default_request_builder(
        &self,
        url: String,
        method: RequestMethod,
    ) -> RequestPayloadBuilder {
        let mut headers = self.req_headers.clone();
        headers.insert(
            reqwest::header::AUTHORIZATION.to_string(),
            format!("Bearer {}", self.access_token.read().await),
        );
        let mut builder = RequestPayloadBuilder::default();
        if let Some(url) = &self.proxy {
            builder.proxy_url(url.to_string());
        }
        builder
            .request_url(url)
            .request_method(method)
            .timeout_seconds(self.timeout.as_secs() as u32)
            .without_cookie_jar(self.cookie_store)
            .headers(headers);
        builder
    }

    async fn response_handle<T: DeserializeOwned>(&self, resp: ResponsePayload) -> ApiResult<T> {
        let status = resp.status();
        if status.is_success() {
            Ok(resp
                .json::<T>()
                .map_err(ApiError::AnyhowJsonDeserializeError)?)
        } else {
            let message = format!("status_code: {:?}, error: {}, ", status, resp.text()?);
            Err(self.error_handle(status, message).await)
        }
    }

    async fn response_stream_handle<T: de::DeserializeOwned>(
        &self,
        resp: ResponsePayload,
    ) -> ApiResult<Box<dyn StreamLine<T>>> {
        let status = resp.status();
        if status.is_success() {
            Ok(Box::new(StreamResponseWrapper(resp)))
        } else {
            let message = format!("status_code: {:?}, error: {}, ", status, resp.text()?);
            Err(self.error_handle(status, message).await)
        }
    }

    async fn error_handle(&self, status: StatusCode, message: String) -> ApiError {
        if status.is_client_error() {
            return ApiError::BadRequest(message);
        }

        if status.is_server_error() {
            return ApiError::ServerError;
        }

        if status.is_redirection() {
            return ApiError::RedirectionError;
        }

        ApiError::FailedRequest(message)
    }
}
impl RefreshToken for OpenGPT {
    fn refresh_token(&mut self, access_token: String) {
        self.access_token = RwLock::new(access_token)
    }
}

#[async_trait]
impl Api for OpenGPT {
    async fn get_models(&self) -> ApiResult<resp::ModelsResponse> {
        let url = format!("{URL_CHATGPT_BASE}/backend-api/models");
        let payload = self
            .default_request_builder(url, RequestMethod::GET)
            .await
            .build()
            .map_err(|op| ApiError::FailedRequest(op.to_string()))?;
        let resp = fficall::request(payload)?;
        self.response_handle::<resp::ModelsResponse>(resp).await
    }

    async fn account_check(&self) -> ApiResult<resp::AccountsCheckResponse> {
        let url = format!("{URL_CHATGPT_BASE}/backend-api/accounts/check");
        let payload = self
            .default_request_builder(url, RequestMethod::GET)
            .await
            .build()
            .map_err(|op| ApiError::FailedRequest(op.to_string()))?;
        let resp = fficall::request(payload)?;
        self.response_handle::<resp::AccountsCheckResponse>(resp)
            .await
    }

    async fn get_conversation(
        &self,
        req: req::GetConversationRequest,
    ) -> ApiResult<resp::GetConversationResonse> {
        let url = format!(
            "{URL_CHATGPT_BASE}/backend-api/conversation/{}",
            req.conversation_id
        );
        let payload = self
            .default_request_builder(url, RequestMethod::GET)
            .await
            .build()
            .map_err(|op| ApiError::FailedRequest(op.to_string()))?;
        let resp = fficall::request(payload)?;
        self.response_handle::<resp::GetConversationResonse>(resp)
            .await
    }

    async fn get_conversations(
        &self,
        req: req::GetConversationRequest,
    ) -> ApiResult<resp::GetConversationsResponse> {
        let url = format!(
            "{URL_CHATGPT_BASE}/backend-api/conversations?offset={}&limit={}",
            req.offset, req.limit
        );
        let payload = self
            .default_request_builder(url, RequestMethod::GET)
            .await
            .build()
            .map_err(|op| ApiError::FailedRequest(op.to_string()))?;
        let resp = fficall::request(payload)?;
        self.response_handle::<resp::GetConversationsResponse>(resp)
            .await
    }

    async fn create_conversation(
        &self,
        req: req::CreateConversationRequest,
    ) -> ApiResult<Box<dyn StreamLine<resp::CreateConversationResponse>>> {
        let url = format!("{URL_CHATGPT_BASE}/backend-api/conversation");
        let body = serde_json::to_string(&req)?;
        drop(req);
        let payload = self
            .default_request_builder(url, RequestMethod::POST)
            .await
            .request_body(body)
            .build()
            .map_err(|op| ApiError::FailedRequest(op.to_string()))?;
        let resp = fficall::request_stream(payload)?;
        self.response_stream_handle(resp).await
    }

    async fn clear_conversation(
        &self,
        req: req::ClearConversationRequest,
    ) -> ApiResult<resp::ClearConversationResponse> {
        let url = format!(
            "{URL_CHATGPT_BASE}/backend-api/conversation/{}",
            req.conversation_id
        );
        let body = serde_json::to_string(&req)?;
        drop(req);
        let payload = self
            .default_request_builder(url, RequestMethod::PATCH)
            .await
            .request_body(body)
            .build()
            .map_err(|op| ApiError::FailedRequest(op.to_string()))?;
        let resp = fficall::request(payload)?;
        self.response_handle::<resp::ClearConversationResponse>(resp)
            .await
    }

    async fn clear_conversations(
        &self,
        req: req::ClearConversationRequest,
    ) -> ApiResult<resp::ClearConversationResponse> {
        let url = format!("{URL_CHATGPT_BASE}/backend-api/conversations");
        let body = serde_json::to_string(&req)?;
        drop(req);
        let payload = self
            .default_request_builder(url, RequestMethod::PATCH)
            .await
            .request_body(body)
            .build()
            .map_err(|op| ApiError::FailedRequest(op.to_string()))?;
        let resp = fficall::request(payload)?;
        self.response_handle::<resp::ClearConversationResponse>(resp)
            .await
    }

    async fn rename_conversation(
        &self,
        req: req::RenameConversationRequest,
    ) -> ApiResult<resp::RenameConversationResponse> {
        let url = format!(
            "{URL_CHATGPT_BASE}/backend-api/conversation/{}",
            req.conversation_id
        );
        let body =
            serde_json::to_string(&req).map_err(|err| ApiError::SerializeError(err.to_string()))?;
        drop(req);
        let payload = self
            .default_request_builder(url, RequestMethod::PATCH)
            .await
            .request_body(body)
            .build()
            .map_err(|op| ApiError::FailedRequest(op.to_string()))?;
        let resp = fficall::request(payload)?;
        self.response_handle::<resp::RenameConversationResponse>(resp)
            .await
    }

    async fn message_feedback(
        &self,
        req: req::MessageFeedbackRequest,
    ) -> ApiResult<resp::MessageFeedbackResponse> {
        let url = format!("{URL_CHATGPT_BASE}/backend-api/conversation/message_feedback",);
        let body = serde_json::to_string(&req).map_err(ApiError::SerdeDeserializeError)?;
        drop(req);
        let payload = self
            .default_request_builder(url, RequestMethod::POST)
            .await
            .request_body(body)
            .build()
            .map_err(|op| ApiError::FailedRequest(op.to_string()))?;
        let resp = fficall::request(payload)?;
        self.response_handle::<resp::MessageFeedbackResponse>(resp)
            .await
    }
}

pub struct OpenGPTBuilder {
    proxy: Option<Url>,
    timeout: Duration,
    cookie_store: bool,
    access_token: String,
}

impl OpenGPTBuilder {
    pub fn proxy(mut self, proxy: Option<Url>) -> Self {
        self.proxy = proxy;
        self
    }

    pub fn client_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn cookie_store(mut self, store: bool) -> Self {
        self.cookie_store = store;
        self
    }

    pub fn access_token(mut self, access_token: String) -> Self {
        self.access_token = access_token;
        self
    }

    pub fn build(self) -> OpenGPT {
        let mut req_headers = HashMap::new();
        req_headers.insert(reqwest::header::USER_AGENT.to_string(), UA.to_string());
        OpenGPT {
            access_token: RwLock::new(self.access_token),
            req_headers: req_headers,
            proxy: self.proxy,
            timeout: self.timeout,
            cookie_store: self.cookie_store,
        }
    }

    pub fn builder() -> OpenGPTBuilder {
        OpenGPTBuilder {
            proxy: None,
            timeout: std::time::Duration::from_secs(30),
            cookie_store: false,
            access_token: String::new(),
        }
    }
}
