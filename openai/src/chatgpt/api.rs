use std::{
    ops::Add,
    time::{Duration, SystemTime},
};

use anyhow::Context;
use async_trait::async_trait;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Proxy, StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::debug;

use super::{
    models::{req, resp},
    Api, ApiError, ApiResult, Method, ToConversationID,
};

const HEADER_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36";
const URL_CHAT_BASE: &str = "https://ai.fakeopen.com";

pub struct ChatApi {
    client: reqwest::Client,
    access_token: RwLock<String>,
    expires: RwLock<Option<SystemTime>>,
}

impl ChatApi {
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
            _ => anyhow::bail!("not supported method"),
        }
        .bearer_auth(token)
        .json(payload);
        self.request_handle::<U>(builder).await
    }

    async fn request_handle<U: DeserializeOwned>(
        &self,
        builder: reqwest::RequestBuilder,
    ) -> ApiResult<U> {
        let resp = builder.send().await?;
        let url = resp.url().clone();
        match resp.error_for_status_ref() {
            Ok(_) => Ok(resp.json::<U>().await.context(ApiError::DeserializeError)?),
            Err(err) => {
                let err_msg = resp.text().await?;
                println!("error: {}, url: {}", err_msg, url);
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
                                anyhow::bail!(ApiError::FailedAuthenticationError)
                            }
                            if status_code.is_client_error() {
                                anyhow::bail!(ApiError::FailedRequest)
                            }
                            anyhow::bail!(ApiError::ServerError)
                        },
                        _ => anyhow::bail!(err),
                    }
            }
        }
    }

    async fn device_check(&self) -> ApiResult<()> {
        use std::time::{Instant, UNIX_EPOCH};

        let last_checked = self.expires.read().await;
        let expired = if let Some(expired_time) = *last_checked {
            let expired_time_timestamp = expired_time
                .duration_since(UNIX_EPOCH)
                .context(ApiError::SystemTimeExceptionError)?
                .as_secs();

            // Confirm half an hour in advance
            let now_timestamp = Instant::now().elapsed().as_secs().add(3000);

            expired_time_timestamp < now_timestamp
        } else {
            true
        };
        drop(last_checked);
        if expired {
            let payload = DeviceCheckPayloadBuilder::default().build()?;

            let token = self.access_token.read().await;
            let url = format!("{URL_CHAT_BASE}/api/devicecheck");
            let resp = self
                .client
                .post(&url)
                .bearer_auth(token)
                .json(&payload)
                .send()
                .await?;
            match resp.error_for_status_ref() {
                Ok(resp) => {
                    if let Some(cookie) = resp.cookies().find(|ele| ele.name().eq("_devicecheck")) {
                        debug!("cookie value: {:?}", cookie.value());
                        debug!("cookie expires: {:?}", cookie.expires());
                        let mut expires = self.expires.write().await;
                        *expires = cookie.expires();
                    }
                }
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
                                anyhow::bail!(ApiError::FailedAuthenticationError)
                            }
                            if status_code.is_client_error() {
                                anyhow::bail!(ApiError::FailedRequest)
                            }
                            anyhow::bail!(ApiError::ServerError)
                        }
                        _ => anyhow::bail!(err),
                    }
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Api for ChatApi {
    async fn get_models(&self) -> ApiResult<resp::ModelsResponse> {
        self.request(format!("{URL_CHAT_BASE}/api/models"), Method::GET)
            .await
    }

    async fn account_check(&self) -> ApiResult<resp::AccountsCheckResponse> {
        self.request(format!("{URL_CHAT_BASE}/api/accounts/check"), Method::GET)
            .await
    }

    async fn get_conversation(
        &self,
        conversation_id: &str,
    ) -> ApiResult<resp::GetConversationResonse> {
        self.request::<resp::GetConversationResonse>(
            format!("{URL_CHAT_BASE}/api/conversation/{conversation_id}"),
            Method::GET,
        )
        .await
    }

    async fn get_conversations(&self) -> ApiResult<resp::GetConversationsResponse> {
        todo!()
    }

    async fn create_conversation(&self, _payload: req::CreateConversationRequest) -> ApiResult<()> {
        self.device_check().await?;
        // self.request_payload(
        //     format!("{}/api/conversation", URL_IOS_CHAT_BASE),
        //     Method::POST,
        //     &payload,
        // )
        // .await
        // .and(Ok(()))
        todo!()
    }

    async fn delete_conversation(
        &self,
        payload: req::DeleteConversationRequest,
    ) -> ApiResult<resp::DeleteConversationResponse> {
        self.request_payload(
            format!(
                "{URL_CHAT_BASE}/api/conversation/{}",
                payload.to_conversation_id()
            ),
            Method::PATCH,
            &payload,
        )
        .await
    }

    async fn delete_conversations(&self, payload: req::DeleteConversationRequest) -> ApiResult<()> {
        self.request_payload(
            format!("{URL_CHAT_BASE}/api/conversations"),
            Method::PATCH,
            &payload,
        )
        .await
    }

    async fn rename_conversation(
        &self,
        payload: req::RenameConversationRequest,
    ) -> ApiResult<resp::RenameConversationResponse> {
        self.request_payload(
            format!(
                "{URL_CHAT_BASE}/api/conversation/{}",
                payload.to_conversation_id()
            ),
            Method::PATCH,
            &payload,
        )
        .await
    }
}

impl super::RefreshToken for ChatApi {
    fn refresh_token(&mut self, access_token: String) {
        self.access_token = RwLock::new(access_token)
    }
}

pub struct IosChatApiBuilder {
    builder: reqwest::ClientBuilder,
    api: ChatApi,
}

impl<'a> IosChatApiBuilder {
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

    pub fn build(mut self) -> ChatApi {
        self.api.client = self.builder.build().expect("ClientBuilder::build()");
        self.api
    }

    pub fn builder() -> IosChatApiBuilder {
        let mut req_headers = HeaderMap::new();
        req_headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_static(HEADER_UA),
        );

        let client = reqwest::ClientBuilder::new()
            .cookie_store(true)
            .default_headers(req_headers);

        IosChatApiBuilder {
            builder: client,
            api: ChatApi {
                client: reqwest::Client::new(),
                expires: RwLock::default(),
                access_token: RwLock::default(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Debug, derive_builder::Builder)]
struct DeviceCheckPayload {
    device_token: String,
    bundle_id: String,
}
