use std::{
    ops::Add,
    time::{Duration, SystemTime},
};

use anyhow::Context;
use async_trait::async_trait;
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Proxy, StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::debug;

use super::{
    models::{req, resp},
    Api, ApiError, ApiResult, Method, ToConversationID,
};

const HEADER_OAI_DEVICE_ID: &str = "0E92DAF9-94F0-4F77-BDF4-53A60D19EC65";
const HEADER_OAI_CLIENT: &str = "ios";
const HEADER_UA: &str = "ChatGPT/1.2023.21 (iOS 16.2; iPad11,1; build 623)";
const HEADER_HOST: &str = "ios.chat.openai.com";
const URL_IOS_CHAT_BASE: &str = "https://ios.chat.openai.com";
const DEVICE_TOKEN: &str = "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";
const BUNDLE_ID: &str = "com.openai.chat";

pub struct IosChatApi {
    client: reqwest::Client,
    access_token: RwLock<String>,
    expires: RwLock<Option<SystemTime>>,
}

impl IosChatApi {
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
        self.device_check().await?;
        let resp = builder.send().await?;
        let url = resp.url().clone();
        match resp.error_for_status_ref() {
            Ok(_) => Ok(resp.json::<U>().await.context(ApiError::DeserializeError)?),
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
            let payload = DeviceCheckPayloadBuilder::default()
                .device_token(DEVICE_TOKEN.to_string())
                .bundle_id(BUNDLE_ID.to_string())
                .build()?;

            let token = self.access_token.read().await;
            let url = format!("{URL_IOS_CHAT_BASE}/backend-api/devicecheck");
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
                        println!("{:?}", cookie.expires())
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
impl Api for IosChatApi {
    async fn get_models(&self) -> ApiResult<resp::ModelsResponse> {
        self.request(
            format!("{URL_IOS_CHAT_BASE}/backend-api/models"),
            Method::GET,
        )
        .await
    }

    async fn account_check(&self) -> ApiResult<resp::AccountsCheckResponse> {
        self.request(
            format!("{URL_IOS_CHAT_BASE}/backend-api/accounts/check"),
            Method::GET,
        )
        .await
    }

    async fn get_conversation(
        &self,
        conversation_id: &str,
    ) -> ApiResult<resp::GetConversationResonse> {
        self.request::<resp::GetConversationResonse>(
            format!("{URL_IOS_CHAT_BASE}/backend-api/conversation/{conversation_id}"),
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
        //     format!("{}/backend-api/conversation", URL_IOS_CHAT_BASE),
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
                "{URL_IOS_CHAT_BASE}/backend-api/conversation/{}",
                payload.to_conversation_id()
            ),
            Method::PATCH,
            &payload,
        )
        .await
    }

    async fn delete_conversations(&self, payload: req::DeleteConversationRequest) -> ApiResult<()> {
        self.request_payload(
            format!("{URL_IOS_CHAT_BASE}/backend-api/conversations"),
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
                "{URL_IOS_CHAT_BASE}/backend-api/conversation/{}",
                payload.to_conversation_id()
            ),
            Method::PATCH,
            &payload,
        )
        .await
    }
}

impl super::RefreshToken for IosChatApi {
    fn refresh_token(&mut self, access_token: String) {
        self.access_token = RwLock::new(access_token)
    }
}

pub struct IosChatApiBuilder {
    builder: reqwest::ClientBuilder,
    api: IosChatApi,
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

    pub fn build(mut self) -> IosChatApi {
        self.api.client = self.builder.build().expect("ClientBuilder::build()");
        self.api
    }

    pub fn builder() -> IosChatApiBuilder {
        let mut req_headers = HeaderMap::new();
        req_headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_static(HEADER_UA),
        );
        req_headers.insert(header::HOST, HeaderValue::from_static(HEADER_HOST));
        req_headers.insert("OAI-Client", HeaderValue::from_static(HEADER_OAI_CLIENT));
        req_headers.insert(
            "OAI-Device-Id",
            HeaderValue::from_static(HEADER_OAI_DEVICE_ID),
        );

        let client = reqwest::ClientBuilder::new()
            .cookie_store(true)
            .default_headers(req_headers);

        IosChatApiBuilder {
            builder: client,
            api: IosChatApi {
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
