use std::time::Duration;

use async_trait::async_trait;
use reqwest_impersonate::{
    header::{HeaderMap, HeaderValue},
    Proxy, StatusCode,
};
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::RwLock;

use crate::debug;

use super::{
    models::{req, resp},
    Api, ApiError, ApiResult, Method,
};

const HEADER_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36";
const URL_CHATGPT_BASE: &str = "https://ai.fakeopen.com/api";

pub struct ChatGPT {
    bash_url: String,
    client: reqwest_impersonate::Client,
    access_token: RwLock<String>,
}

impl ChatGPT {
    async fn request<U>(&self, url: String, method: Method) -> ApiResult<U>
    where
        U: DeserializeOwned,
    {
        let token = self.access_token.read().await;
        let builder = match method {
            Method::GET => self.client.get(&url),
            Method::POST => self.client.post(&url),
            Method::PATCH => self.client.patch(&url),
            Method::PUT => self.client.put(&url),
            Method::DELETE => self.client.delete(&url),
        }
        .bearer_auth(token);
        self.request_handle(builder).await
    }

    async fn request_payload<T, U>(&self, url: String, method: Method, payload: &T) -> ApiResult<U>
    where
        T: Serialize + ?Sized,
        U: DeserializeOwned,
    {
        let token = self.access_token.read().await;
        let builder = match method {
            Method::POST => self.client.post(&url),
            Method::PATCH => self.client.patch(&url),
            Method::PUT => self.client.put(&url),
            Method::DELETE => self.client.delete(&url),
            _ => return Err(ApiError::FailedRequest("not supported method".to_owned())),
        }
        .bearer_auth(token)
        .json(payload);
        self.request_handle::<U>(builder).await
    }

    async fn request_handle<U: DeserializeOwned>(
        &self,
        builder: reqwest_impersonate::RequestBuilder,
    ) -> ApiResult<U> {
        let resp = builder.send().await?;
        let url = resp.url().clone();
        match resp.error_for_status_ref() {
            Ok(_) => Ok(resp
                .json::<U>()
                .await
                .map_err(ApiError::ReqwestJsonDeserializeError)?),
            Err(err) => {
                let err_msg = resp.text().await?;
                debug!("error: {}, url: {}", err_msg, url);
                match err.status() {
                        Some(
                            status_code
                            @
                            // 4xx
                            (StatusCode::UNAUTHORIZED
                            | StatusCode::REQUEST_TIMEOUT
                            | StatusCode::TOO_MANY_REQUESTS
                            // 5xx
                            | StatusCode::INTERNAL_SERVER_ERROR
                            | StatusCode::BAD_GATEWAY
                            | StatusCode::SERVICE_UNAVAILABLE
                            | StatusCode::GATEWAY_TIMEOUT),
                        ) => {
                            if status_code == StatusCode::UNAUTHORIZED {
                                return Err(ApiError::BadAuthenticationError(err_msg));
                            }
                            if status_code.is_client_error() {
                                return Err(ApiError::BadRequest(err_msg))
                            }
                            Err(ApiError::ServerError)
                        },
                        _ => Err(ApiError::FailedRequest(err.to_string())),
                    }
            }
        }
    }
}

#[async_trait]
impl Api for ChatGPT {
    async fn get_models(&self) -> ApiResult<resp::ModelsResponse> {
        self.request(format!("{URL_CHATGPT_BASE}/models"), Method::GET)
            .await
    }

    async fn account_check(&self) -> ApiResult<resp::AccountsCheckResponse> {
        self.request(format!("{URL_CHATGPT_BASE}/accounts/check"), Method::GET)
            .await
    }

    async fn get_conversation(
        &self,
        req: req::GetConversationRequest,
    ) -> ApiResult<resp::GetConversationResonse> {
        self.request::<resp::GetConversationResonse>(
            format!("{URL_CHATGPT_BASE}/conversation/{}", req.conversation_id),
            Method::GET,
        )
        .await
    }

    async fn get_conversations(
        &self,
        req: req::GetConversationRequest,
    ) -> ApiResult<resp::GetConversationsResponse> {
        self.request::<resp::GetConversationsResponse>(
            format!(
                "{URL_CHATGPT_BASE}/conversation?offset={}&limit={}",
                req.offset, req.limit
            ),
            Method::GET,
        )
        .await
    }

    async fn create_conversation(
        &self,
        _req: req::CreateConversationRequest,
    ) -> ApiResult<resp::CreateConversationResponse> {
        // self.request_payload(
        //     format!("{}/api/conversation", URL_IOS_CHAT_BASE),
        //     Method::POST,
        //     &payload,
        // )
        // .await
        // .and(Ok(()))
        todo!()
    }

    async fn clear_conversation(
        &self,
        req: req::ClearConversationRequest,
    ) -> ApiResult<resp::ClearConversationResponse> {
        self.request_payload(
            format!("{URL_CHATGPT_BASE}/conversation/{}", req.conversation_id),
            Method::PATCH,
            &req,
        )
        .await
    }

    async fn clear_conversations(
        &self,
        req: req::ClearConversationRequest,
    ) -> ApiResult<resp::ClearConversationResponse> {
        self.request_payload(
            format!("{URL_CHATGPT_BASE}/conversations"),
            Method::PATCH,
            &req,
        )
        .await
    }

    async fn rename_conversation(
        &self,
        req: req::RenameConversationRequest,
    ) -> ApiResult<resp::RenameConversationResponse> {
        self.request_payload(
            format!(
                "{URL_CHATGPT_BASE}/api/conversation/{}",
                req.conversation_id
            ),
            Method::PATCH,
            &req,
        )
        .await
    }

    async fn message_feedback(
        &self,
        req: req::MessageFeedbackRequest,
    ) -> ApiResult<resp::MessageFeedbackResponse> {
        self.request_payload(
            format!("{URL_CHATGPT_BASE}/api/conversation/message_feedbak"),
            Method::POST,
            &req,
        )
        .await
    }
}

impl super::RefreshToken for ChatGPT {
    fn refresh_token(&mut self, access_token: String) {
        self.access_token = RwLock::new(access_token)
    }
}

pub struct ChatGPTBuilder {
    builder: reqwest_impersonate::ClientBuilder,
    api: ChatGPT,
}

impl ChatGPTBuilder {
    pub fn base_url(mut self, url: Option<String>) -> Self {
        if let Some(url) = url {
            self.api.bash_url = url
        }
        self
    }

    pub fn proxy(mut self, proxy: Option<Proxy>) -> Self {
        if let Some(proxy) = proxy {
            self.builder = self.builder.proxy(proxy);
        } else {
            self.builder = self.builder.no_proxy();
        }
        self
    }

    pub fn client_timeout(mut self, timeout: Duration) -> Self {
        self.builder = self.builder.timeout(timeout);
        self
    }

    pub fn client_connect_timeout(mut self, timeout: Duration) -> Self {
        self.builder = self.builder.connect_timeout(timeout);
        self
    }

    pub fn cookie_store(mut self, store: bool) -> Self {
        self.builder = self.builder.cookie_store(store);
        self
    }

    pub fn access_token(mut self, access_token: String) -> Self {
        self.api.access_token = tokio::sync::RwLock::new(access_token);
        self
    }

    pub fn build(mut self) -> ChatGPT {
        self.api.client = self.builder.build().expect("ClientBuilder::build()");
        self.api
    }

    pub fn builder() -> ChatGPTBuilder {
        let mut req_headers = HeaderMap::new();
        req_headers.insert(
            reqwest_impersonate::header::USER_AGENT,
            HeaderValue::from_static(HEADER_UA),
        );

        let client = reqwest_impersonate::ClientBuilder::new()
            .cookie_store(true)
            .default_headers(req_headers);

        ChatGPTBuilder {
            builder: client,
            api: ChatGPT {
                bash_url: String::from(URL_CHATGPT_BASE),
                client: reqwest_impersonate::Client::new(),
                access_token: RwLock::default(),
            },
        }
    }
}
