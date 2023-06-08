extern crate regex;

use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

use async_recursion::async_recursion;
use regex::Regex;
use reqwest_impersonate::header::{HeaderMap, HeaderValue};
use reqwest_impersonate::redirect::Policy;
use serde::Deserialize;
use serde_json::json;

use base64::{engine::general_purpose, Engine as _};
use rand::Rng;
use reqwest_impersonate::{Client, Proxy, Url};
use sha2::{Digest, Sha256};

use crate::{token, OAuthError, OAuthResult};

const UA: &str = "ChatGPT/1.2023.21 (iOS 16.2; iPad11,1; build 623)";
const CLIENT_ID: &str = "pdlLIX2Y72MIl2rhLhTE9VV9bN905kBh";
const OPENAI_OAUTH_URL: &str = "https://auth0.openai.com";
const OPENAI_OAUTH_TOKEN_URL: &str = "https://auth0.openai.com/oauth/token";
const OPENAI_OAUTH_REVOKE_URL: &str = "https://auth0.openai.com/oauth/revoke";
const OPENAI_OAUTH_CALLBACK_URL: &str =
    "com.openai.chat://auth0.openai.com/ios/com.openai.chat/callback";

pub struct OAuth {
    email: String,
    session: Client,
    password: String,
    mfa: Option<String>,
    cache: bool,
    req_headers: HeaderMap,
    email_regex: Regex,
    store: Box<dyn token::AuthenticateTokenStore>,
}

// api: https://auth0.openai.com
impl OAuth {
    fn generate_code_verifier() -> String {
        let token: [u8; 32] = rand::thread_rng().gen();
        let code_verifier = general_purpose::URL_SAFE
            .encode(token)
            .trim_end_matches('=')
            .to_string();
        code_verifier
    }

    fn generate_code_challenge(code_verifier: &str) -> String {
        let mut m = Sha256::new();
        m.update(code_verifier.as_bytes());
        let code_challenge = general_purpose::URL_SAFE
            .encode(m.finalize())
            .trim_end_matches('=')
            .to_string();
        code_challenge
    }

    fn get_callback_code(url: &Url) -> OAuthResult<String> {
        let mut url_params = HashMap::new();
        url.query_pairs().into_owned().for_each(|(key, value)| {
            url_params
                .entry(key)
                .and_modify(|v: &mut Vec<String>| v.push(value.clone()))
                .or_insert(vec![value]);
        });

        if let Some(error) = url_params.get("error") {
            if let Some(error_description) = url_params.get("error_description") {
                let msg = format!("{}: {}", error[0], error_description[0]);
                anyhow::bail!("{}", msg);
            } else {
                anyhow::bail!("{}", error[0]);
            }
        }

        let code = url_params
            .get("code")
            .ok_or(OAuthError::FailedCallbackCode)?[0]
            .to_string();
        Ok(code)
    }

    fn get_callback_state(url: &Url) -> String {
        let url_params = url.query_pairs().into_owned().collect::<HashMap<_, _>>();
        url_params["state"].to_owned()
    }

    pub async fn do_get_access_token(&mut self) -> OAuthResult<token::AuthenticateToken> {
        let token = self.store.get_token(&self.email).await?;
        if self.cache && token.is_some() {
            return Ok(token.ok_or(OAuthError::FailedLoginIn).and_then(|op| {
                if op.is_expired() {
                    return Err(OAuthError::FailedLoginIn);
                }
                Ok(op)
            })?);
        }

        if !self.email_regex.is_match(&self.email) || self.password.is_empty() {
            anyhow::bail!(OAuthError::InvalidEmailOrPassword)
        }
        self.login_handler().await
    }

    async fn login_handler(&mut self) -> OAuthResult<token::AuthenticateToken> {
        let code_verifier = Self::generate_code_verifier();
        let code_challenge = Self::generate_code_challenge(&code_verifier);

        let url = format!("https://auth0.openai.com/authorize?client_id={}&audience=https%3A%2F%2Fapi.openai.com%2Fv1&redirect_uri=com.openai.chat%3A%2F%2Fauth0.openai.com%2Fios%2Fcom.openai.chat%2Fcallback&scope=openid%20email%20profile%20offline_access%20model.request%20model.read%20organization.read%20organization.write%20offline&response_type=code&code_challenge={}&code_challenge_method=S256&prompt=login", CLIENT_ID, code_challenge);

        let mut headers = self.req_headers.clone();
        headers.insert(
            reqwest_impersonate::header::REFERER,
            HeaderValue::from_static(OPENAI_OAUTH_URL),
        );
        let resp = self.session.get(url).headers(headers).send().await?;

        if resp.status().is_success() {
            let state = Self::get_callback_state(resp.url());
            self.login_handler0(&code_verifier, &state).await
        } else {
            anyhow::bail!(OAuthError::InvalidLoginUrl)
        }
    }

    async fn login_handler0(
        &mut self,
        code_verifier: &str,
        state: &str,
    ) -> OAuthResult<token::AuthenticateToken> {
        let url = format!(
            "https://auth0.openai.com/u/login/identifier?state={}",
            state
        );
        let mut headers = self.req_headers.clone();
        headers.insert(reqwest_impersonate::header::REFERER, HeaderValue::from_str(&url)?);
        headers.insert(
            reqwest_impersonate::header::ORIGIN,
            HeaderValue::from_static(OPENAI_OAUTH_URL),
        );
        let data = json!({
            "state": state,
            "username": self.email.to_string(),
            "js-available": true,
            "webauthn-available": true,
            "is-brave": false,
            "webauthn-platform-available": false,
            "action": "default",
        });
        let resp = self
            .session
            .post(&url)
            .headers(headers)
            .json(&data)
            .send()
            .await?;

        if resp.status().is_redirection() {
            let location = resp
                .headers()
                .get("Location")
                .ok_or(OAuthError::InvalidLocation)?
                .to_str()?;
            self.login_handler1(code_verifier, state, location, &url)
                .await
        } else {
            anyhow::bail!(OAuthError::InvalidEmail)
        }
    }

    async fn login_handler1(
        &mut self,
        code_verifier: &str,
        state: &str,
        location: &str,
        referrer: &str,
    ) -> OAuthResult<token::AuthenticateToken> {
        let url = format!("{}{}", OPENAI_OAUTH_URL, location);
        let mut headers = self.req_headers.clone();
        headers.insert(reqwest_impersonate::header::REFERER, HeaderValue::from_str(referrer)?);
        let data = json!({
            "state": state,
            "username": self.email.to_string(),
            "password": self.password.to_string(),
            "action": "default"
        });
        let resp = self
            .session
            .post(&url)
            .headers(headers)
            .json(&data)
            .send()
            .await?;

        if resp.status().is_redirection() {
            let location = resp
                .headers()
                .get("Location")
                .ok_or(OAuthError::InvalidLocation)?
                .to_str()?;
            if !location.starts_with("/authorize/resume?") {
                anyhow::bail!(OAuthError::FailedLogin)
            }
            self.login_handler2(code_verifier, location, &url).await
        } else if resp.status().is_client_error() {
            anyhow::bail!(OAuthError::InvalidEmailOrPassword)
        } else {
            anyhow::bail!(OAuthError::FailedLogin)
        }
    }

    #[async_recursion]
    async fn login_handler2(
        &mut self,
        code_verifier: &str,
        location: &str,
        referrer: &str,
    ) -> OAuthResult<token::AuthenticateToken> {
        let url = format!("{}{}", OPENAI_OAUTH_URL, location);
        let mut headers = self.req_headers.clone();
        headers.insert(
            reqwest_impersonate::header::REFERER,
            HeaderValue::from_str(referrer).unwrap(),
        );
        let resp = self.session.get(&url).headers(headers).send().await?;

        if resp.status().is_redirection() {
            let location = resp
                .headers()
                .get("Location")
                .ok_or(OAuthError::InvalidLocation)?
                .to_str()?;
            if location.starts_with("/u/mfa-otp-challenge?") {
                if self.mfa.is_none() {
                    anyhow::bail!(OAuthError::MFARequired)
                }
                return self.login_handler3(code_verifier, location).await;
            } else if !location.starts_with(OPENAI_OAUTH_CALLBACK_URL) {
                anyhow::bail!(OAuthError::FailedCallbackURL)
            } else {
                return self.login_handler4(code_verifier, location).await;
            }
        }
        anyhow::bail!(OAuthError::FailedLogin)
    }

    #[async_recursion]
    async fn login_handler3(
        &mut self,
        code_verifier: &str,
        location: &str,
    ) -> OAuthResult<token::AuthenticateToken> {
        let url = format!("{}{}", OPENAI_OAUTH_URL, location);
        let state = Self::get_callback_state(&Url::parse(&url)?);
        let data = json!({
            "state": state,
            "code": self.mfa.clone().ok_or(OAuthError::MFARequired)?,
            "action": "default"
        });

        let mut headers = self.req_headers.clone();
        headers.insert(reqwest_impersonate::header::REFERER, HeaderValue::from_str(&url)?);
        headers.insert(
            reqwest_impersonate::header::ORIGIN,
            HeaderValue::from_static(OPENAI_OAUTH_URL),
        );
        headers.insert(reqwest_impersonate::header::USER_AGENT, HeaderValue::from_static(UA));

        let resp = self
            .session
            .post(&url)
            .json(&data)
            .headers(headers)
            .send()
            .await?;
        let status = resp.status();
        if status.is_redirection() {
            let location = resp
                .headers()
                .get("Location")
                .ok_or(OAuthError::InvalidLocation)?
                .to_str()?;

            if location.starts_with("/authorize/resume?") {
                if self.mfa.is_none() {
                    anyhow::bail!(OAuthError::MFAFailed)
                }
            }
            return self.login_handler2(code_verifier, location, &url).await;
        }
        if status.is_client_error() {
            anyhow::bail!(OAuthError::InvalidMFACode)
        }
        anyhow::bail!(OAuthError::FailedLogin)
    }

    async fn login_handler4(
        &mut self,
        code_verifier: &str,
        callback_url: &str,
    ) -> OAuthResult<token::AuthenticateToken> {
        let url = Url::parse(callback_url)?;
        let code = Self::get_callback_code(&url)?;
        let data = json!({
            "redirect_uri": OPENAI_OAUTH_CALLBACK_URL.to_string(),
            "grant_type": "authorization_code".to_string(),
            "client_id": CLIENT_ID.to_string(),
            "code": code,
            "code_verifier": code_verifier.to_string()
        });

        let resp = self
            .session
            .post(OPENAI_OAUTH_TOKEN_URL)
            .headers(self.req_headers.clone())
            .json(&data)
            .send()
            .await?;

        if resp.status().is_success() {
            let result = resp.json::<AccessToken>().await?;
            let authentication_token = token::AuthenticateToken::try_from(result)?;
            let access_token = authentication_token.clone();
            self.store
                .set_token(authentication_token)
                .await
                .and(Ok(access_token))
        } else {
            anyhow::bail!(OAuthError::FailedLogin)
        }
    }

    pub async fn do_refresh_token(&mut self) -> OAuthResult<token::AuthenticateToken> {
        let token = self
            .store
            .get_token(&self.email)
            .await?
            .ok_or(OAuthError::FailedLoginIn)?;

        let refresh_token = token.refresh_token();

        let data = json!({
            "redirect_uri": OPENAI_OAUTH_CALLBACK_URL.to_string(),
            "grant_type": "refresh_token".to_string(),
            "client_id": CLIENT_ID.to_string(),
            "refresh_token": refresh_token
        });

        let resp = self
            .session
            .post(OPENAI_OAUTH_TOKEN_URL)
            .json(&data)
            .send()
            .await?;
        if resp.status().is_success() {
            let result = resp.json::<RefreshToken>().await.map(|mut op| {
                op.refresh_token = refresh_token.to_owned();
                op
            })?;

            let authenticate_token = token::AuthenticateToken::try_from(result)?;
            let access_token = authenticate_token.clone();
            self.store
                .set_token(authenticate_token)
                .await
                .and(Ok(access_token))
        } else {
            anyhow::bail!(resp.json::<OAuthError>().await?)
        }
    }

    pub async fn do_revoke_token(&mut self) -> OAuthResult<()> {
        let token = self
            .store
            .get_token(&self.email)
            .await?
            .ok_or(OAuthError::FailedLoginIn)?;
        let data = json!({
            "client_id": CLIENT_ID.to_string(),
            "token": token.refresh_token()
        });
        let resp = self
            .session
            .post(OPENAI_OAUTH_REVOKE_URL)
            .json(&data)
            .send()
            .await?;
        if resp.status().is_success() {
            self.store.delete_token(&self.email).await.and(Ok(()))
        } else {
            anyhow::bail!(resp.json::<OAuthError>().await?)
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AccessToken {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub expires_in: i64,
}

#[derive(Debug, Deserialize)]
pub struct RefreshToken {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    pub id_token: String,
    pub expires_in: i64,
}

pub struct OAuthBuilder {
    builder: reqwest_impersonate::ClientBuilder,
    oauth: OAuth,
}

impl OAuthBuilder {
    pub fn email(mut self, email: String) -> Self {
        self.oauth.email = email;
        self
    }

    pub fn password(mut self, password: String) -> Self {
        self.oauth.password = password;
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

    pub fn cache(mut self, cache: bool) -> Self {
        self.oauth.cache = cache;
        self
    }

    pub fn mfa(mut self, mfa: Option<String>) -> Self {
        self.oauth.mfa = mfa;
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

    pub fn token_store<S: token::AuthenticateTokenStore + 'static>(mut self, store: S) -> Self {
        self.oauth.store = Box::new(store);
        self
    }

    pub fn build(mut self) -> OAuth {
        self.oauth.session = self.builder.build().expect("ClientBuilder::build()");
        self.oauth
    }

    pub fn builder() -> OAuthBuilder {
        let mut req_headers = HeaderMap::new();
        req_headers.insert(reqwest_impersonate::header::USER_AGENT, HeaderValue::from_static(UA));

        let client_builder = Client::builder().redirect(Policy::custom(|attempt| {
            if attempt
                .url()
                .to_string()
                .contains("https://auth0.openai.com/u/login/identifier")
            {
                // redirects to 'https://auth0.openai.com/u/login/identifier'
                attempt.follow()
            } else {
                attempt.stop()
            }
        }));

        OAuthBuilder {
            builder: client_builder,
            oauth: OAuth {
                email: String::new(),
                password: String::new(),
                session: Client::new(),
                cache: false,
                mfa: None,
                req_headers,
                email_regex: Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,7}\b")
                    .expect("Regex::new()"),
                store: Box::new(token::MemStore::new()),
            },
        }
    }
}
