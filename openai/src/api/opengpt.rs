use std::{collections::HashMap, time::Duration};
use reqwest_impersonate::StatusCode;

use crate::api::ApiError;

use super::{
    models::{req, resp},
    Api, ApiResult, RefreshToken,
};
use async_trait::async_trait;

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
        todo!()
    }

    async fn account_check(&self) -> ApiResult<resp::AccountsCheckResponse> {
        let url = format!("{URL_CHATGPT_BASE}/backend-api/accounts/check");
        todo!()
    }

    async fn get_conversation(
        &self,
        req: req::GetConversationRequest,
    ) -> ApiResult<resp::GetConversationResonse> {
        let url = format!(
            "{URL_CHATGPT_BASE}/backend-api/conversation/{}",
            req.conversation_id
        );
        todo!()
    }

    async fn get_conversations(
        &self,
        req: req::GetConversationRequest,
    ) -> ApiResult<resp::GetConversationsResponse> {
        let url = format!(
            "{URL_CHATGPT_BASE}/backend-api/conversations?offset={}&limit={}",
            req.offset, req.limit
        );
        todo!()
    }

    async fn create_conversation(
        &self,
        req: req::CreateConversationRequest,
    ) -> ApiResult<resp::CreateConversationResponse> {
        let url = format!("{URL_CHATGPT_BASE}/backend-api/conversation");
        todo!()
    }

    async fn clear_conversation(
        &self,
        req: req::ClearConversationRequest,
    ) -> ApiResult<resp::ClearConversationResponse> {
        let url = format!(
            "{URL_CHATGPT_BASE}/backend-api/conversation/{}",
            req.conversation_id
        );
        todo!()
    }

    async fn clear_conversations(
        &self,
        req: req::ClearConversationRequest,
    ) -> ApiResult<resp::ClearConversationResponse> {
        let url = format!("{URL_CHATGPT_BASE}/backend-api/conversations");
        todo!()
    }

    async fn rename_conversation(
        &self,
        req: req::RenameConversationRequest,
    ) -> ApiResult<resp::RenameConversationResponse> {
        let url = format!(
            "{URL_CHATGPT_BASE}/backend-api/conversation/{}",
            req.conversation_id
        );
        todo!()
    }

    async fn message_feedback(
        &self,
        req: req::MessageFeedbackRequest,
    ) -> ApiResult<resp::MessageFeedbackResponse> {
        let url = format!("{URL_CHATGPT_BASE}/backend-api/conversation/message_feedback",);
        todo!()
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
        req_headers.insert(reqwest_impersonate::header::USER_AGENT.to_string(), UA.to_string());
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
