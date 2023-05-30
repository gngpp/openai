extern crate regex;

use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

use async_recursion::async_recursion;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::redirect::Policy;
use serde::Deserialize;
use serde_json::json;

use base64::{engine::general_purpose, Engine as _};
use rand::Rng;
use reqwest::{Client, Proxy, Url};
use sha2::{Digest, Sha256};

use crate::{OAuthError, OAuthResult};

const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36";
const CLIENT_ID: &str = "pdlLIX2Y72MIl2rhLhTE9VV9bN905kBh";
const OPENAI_OAUTH_URL: &str = "https://auth0.openai.com";
const OPENAI_OAUTH_TOKEN_URL: &str = "https://auth0.openai.com/oauth/token";
const OPENAI_OAUTH_REVOKE_URL: &str = "https://auth0.openai.com/oauth/revoke";
const OPENAI_OAUTH_CALLBACK_URL: &str =
    "com.openai.chat://auth0.openai.com/ios/com.openai.chat/callback";

pub struct OpenAIOAuthBuilder {
    client_builder: reqwest::ClientBuilder,
    oauth: OpenAIOAuth,
}

impl OpenAIOAuthBuilder {
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
            self.client_builder = self.client_builder.proxy(proxy);
        } else {
            self.client_builder = self.client_builder.no_proxy();
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
        self.client_builder = self.client_builder.timeout(timeout);
        self
    }

    pub fn client_cookie_store(mut self, store: bool) -> Self {
        self.client_builder = self.client_builder.cookie_store(store);
        self
    }

    pub fn build(mut self) -> OpenAIOAuth {
        self.oauth.session = self.client_builder.build().expect("ClientBuilder::build()");
        self.oauth
    }

    pub fn builder() -> OpenAIOAuthBuilder {
        let mut req_headers = HeaderMap::new();
        req_headers.insert(reqwest::header::USER_AGENT, HeaderValue::from_static(UA));

        let mut client_builder = Client::builder();
        client_builder = client_builder.redirect(Policy::custom(|attempt| {
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

        OpenAIOAuthBuilder {
            client_builder,
            oauth: OpenAIOAuth {
                email: String::new(),
                password: String::new(),
                session: Client::new(),
                cache: false,
                mfa: None,
                req_headers,
                access_token: None,
                refresh_token: None,
                id_token: None,
                expires: None,
                email_regex: Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,7}\b")
                    .expect("Regex::new()"),
            },
        }
    }
}

#[derive(Debug)]
pub struct OpenAIOAuth {
    email: String,
    session: Client,
    password: String,
    mfa: Option<String>,
    cache: bool,
    req_headers: HeaderMap,
    access_token: Option<String>,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires: Option<chrono::DateTime<chrono::Utc>>,
    email_regex: Regex,
}

// api: https://auth0.openai.com
impl OpenAIOAuth {
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

    pub fn get_user_info(&self) -> OAuthResult<OpenAIUserInfo> {
        if let Some(id_token) = self.id_token.clone() {
            let split_jwt_strings: Vec<_> = id_token.split('.').collect();
            let jwt_body = split_jwt_strings
                .get(1)
                .ok_or(OAuthError::InvalidAccessToken)?;
            let decoded_jwt_body = general_purpose::URL_SAFE_NO_PAD.decode(jwt_body)?;
            let converted_jwt_body = String::from_utf8(decoded_jwt_body)?;
            let user_info = serde_json::from_str::<OpenAIUserInfo>(&converted_jwt_body)?;
            return Ok(user_info);
        }
        anyhow::bail!(OAuthError::FailedLoginIn)
    }

    pub async fn authenticate(&mut self) -> OAuthResult<String> {
        if self.cache
            && self.access_token.is_some()
            && self.expires.is_some()
            && self.expires.ok_or(OAuthError::TokenExipired)? > chrono::Utc::now()
        {
            return Ok(self.access_token.clone().ok_or(OAuthError::FailedLoginIn)?);
        }

        if !self.email_regex.is_match(&self.email) || self.password.is_empty() {
            anyhow::bail!(OAuthError::InvalidEmailOrPassword)
        }

        self.login().await
    }

    async fn login(&mut self) -> OAuthResult<String> {
        let code_verifier = Self::generate_code_verifier();
        let code_challenge = Self::generate_code_challenge(&code_verifier);

        let url = format!("https://auth0.openai.com/authorize?client_id={}&audience=https%3A%2F%2Fapi.openai.com%2Fv1&redirect_uri=com.openai.chat%3A%2F%2Fauth0.openai.com%2Fios%2Fcom.openai.chat%2Fcallback&scope=openid%20email%20profile%20offline_access%20model.request%20model.read%20organization.read%20offline&response_type=code&code_challenge={}&code_challenge_method=S256&prompt=login", CLIENT_ID, code_challenge);

        let mut headers = self.req_headers.clone();
        headers.insert(
            reqwest::header::REFERER,
            HeaderValue::from_static(OPENAI_OAUTH_URL),
        );
        let resp = self.session.get(url).headers(headers).send().await?;

        if resp.status().is_success() {
            let state = Self::get_callback_state(resp.url());
            self.identifier_handler(&code_verifier, &state).await
        } else {
            anyhow::bail!(OAuthError::InvalidLoginUrl)
        }
    }

    async fn identifier_handler(
        &mut self,
        code_verifier: &str,
        state: &str,
    ) -> anyhow::Result<String> {
        let url = format!(
            "https://auth0.openai.com/u/login/identifier?state={}",
            state
        );
        let mut headers = self.req_headers.clone();
        headers.insert(reqwest::header::REFERER, HeaderValue::from_str(&url)?);
        headers.insert(
            reqwest::header::ORIGIN,
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
            self.login_handler0(code_verifier, state, location, &url)
                .await
        } else {
            anyhow::bail!(OAuthError::InvalidEmail)
        }
    }

    async fn login_handler0(
        &mut self,
        code_verifier: &str,
        state: &str,
        location: &str,
        referrer: &str,
    ) -> OAuthResult<String> {
        let url = format!("{}{}", OPENAI_OAUTH_URL, location);
        let mut headers = self.req_headers.clone();
        headers.insert(reqwest::header::REFERER, HeaderValue::from_str(referrer)?);
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
            self.login_handler1(code_verifier, location, &url).await
        } else if resp.status().is_client_error() {
            anyhow::bail!(OAuthError::InvalidEmailOrPassword)
        } else {
            anyhow::bail!(OAuthError::FailedLogin)
        }
    }

    #[async_recursion]
    async fn login_handler1(
        &mut self,
        code_verifier: &str,
        location: &str,
        referrer: &str,
    ) -> OAuthResult<String> {
        let url = format!("{}{}", OPENAI_OAUTH_URL, location);
        let mut headers = self.req_headers.clone();
        headers.insert(
            reqwest::header::REFERER,
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
                return self.login_handler2(code_verifier, location).await;
            } else if !location.starts_with(OPENAI_OAUTH_CALLBACK_URL) {
                anyhow::bail!(OAuthError::FailedCallbackURL)
            } else {
                return self.do_get_access_token(code_verifier, location).await;
            }
        }
        anyhow::bail!(OAuthError::FailedLogin)
    }

    #[async_recursion]
    async fn login_handler2(&mut self, code_verifier: &str, location: &str) -> OAuthResult<String> {
        let url = format!("{}{}", OPENAI_OAUTH_URL, location);
        let state = Self::get_callback_state(&Url::parse(&url)?);
        let data = json!({
            "state": state,
            "code": self.mfa.clone().ok_or(OAuthError::MFARequired)?,
            "action": "default"
        });

        let mut headers = self.req_headers.clone();
        headers.insert(reqwest::header::REFERER, HeaderValue::from_str(&url)?);
        headers.insert(
            reqwest::header::ORIGIN,
            HeaderValue::from_static(OPENAI_OAUTH_URL),
        );
        headers.insert(reqwest::header::USER_AGENT, HeaderValue::from_static(UA));

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
            return self.login_handler1(code_verifier, location, &url).await;
        }
        if status.is_client_error() {
            anyhow::bail!(OAuthError::InvalidMFACode)
        }
        anyhow::bail!(OAuthError::FailedLogin)
    }

    async fn do_get_access_token(
        &mut self,
        code_verifier: &str,
        callback_url: &str,
    ) -> OAuthResult<String> {
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
            let result = resp.json::<AccessTokenResult>().await?;
            self.expires = Some(
                chrono::Utc::now() + chrono::Duration::seconds(i64::from(result.expires_in))
                    - chrono::Duration::minutes(5),
            );

            let access_token = result.access_token.clone();
            self.access_token = Some(result.access_token);
            self.refresh_token = Some(result.refresh_token);
            self.id_token = Some(result.id_token);
            return Ok(access_token);
        }
        anyhow::bail!(OAuthError::FailedLogin)
    }

    pub async fn do_refresh_token(&mut self) -> OAuthResult<String> {
        let refresh_token = self
            .refresh_token
            .clone()
            .ok_or(OAuthError::FailedLoginIn)?;
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
            let result = resp.json::<RefreshTokenResult>().await?;
            self.expires = Some(
                chrono::Utc::now() + chrono::Duration::seconds(i64::from(result.expires_in))
                    - chrono::Duration::minutes(5),
            );

            let token = result.access_token.clone();
            self.access_token = Some(result.access_token);
            self.id_token = Some(result.id_token);
            return Ok(token);
        }
        anyhow::bail!(resp.json::<OAuthError>().await?)
    }

    pub async fn do_revoke_token(&mut self) -> OAuthResult<()> {
        let refresh_token = self
            .refresh_token
            .clone()
            .ok_or(OAuthError::FailedLoginIn)?;
        let data = json!({
            "client_id": CLIENT_ID.to_string(),
            "token": refresh_token
        });
        let resp = self
            .session
            .post(OPENAI_OAUTH_REVOKE_URL)
            .json(&data)
            .send()
            .await?;
        if !resp.status().is_success() {
            anyhow::bail!(resp.json::<OAuthError>().await?)
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct AccessTokenResult {
    access_token: String,
    refresh_token: String,
    id_token: String,
    expires_in: u32,
}

#[derive(Debug, Deserialize)]
struct RefreshTokenResult {
    access_token: String,
    id_token: String,
    expires_in: u32,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct OpenAIUserInfo {
    nickname: String,
    name: String,
    picture: String,
    updated_at: String,
    email_verified: bool,
    email: String,
    iss: String,
    aud: String,
    iat: i64,
    exp: i64,
    sub: String,
    auth_time: i64,
}
