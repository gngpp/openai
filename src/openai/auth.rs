extern crate regex;

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context, Ok};
use async_recursion::async_recursion;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::redirect::Policy;
use serde_json::json;
use url::Url;

use base64::{engine::general_purpose, Engine as _};
use rand::Rng;
use sha2::{Digest, Sha256};

const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36";
const CLIENT_ID: &str = "pdlLIX2Y72MIl2rhLhTE9VV9bN905kBh";

#[derive(Debug)]
pub struct OpenAIAuth0 {
    email: String,
    session: reqwest::Client,
    password: String,
    mfa: Option<String>,
    use_cache: bool,
    req_headers: HeaderMap,
    access_token: Option<String>,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires: Option<chrono::DateTime<chrono::Utc>>,
    email_regex: regex::Regex,
}

// api: https://auth0.openai.com
impl OpenAIAuth0 {
    pub fn new(
        email: String,
        password: String,
        proxy: Option<url::Url>,
        use_cache: bool,
        mfa: Option<&str>,
    ) -> anyhow::Result<OpenAIAuth0> {
        let mut req_headers = HeaderMap::new();
        req_headers.insert("User-Agent", HeaderValue::from_str(UA)?);

        let mut client_builder = reqwest::Client::builder()
            .cookie_store(true)
            .timeout(Duration::from_secs(1000));

        if let Some(proxy_url) = proxy {
            client_builder = client_builder.proxy(reqwest::Proxy::all(proxy_url.to_string())?);
        } else {
            client_builder = client_builder.no_proxy();
        }

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

        Ok(OpenAIAuth0 {
            email,
            password,
            session: client_builder.build()?,
            use_cache,
            mfa: mfa.map(String::from),
            req_headers,
            access_token: None,
            refresh_token: None,
            id_token: None,
            expires: None,
            email_regex: Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,7}\b")?,
        })
    }

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

    fn get_callback_code(url: &url::Url) -> anyhow::Result<String> {
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
            .context("Error get code from callback url")?[0]
            .to_string();
        Ok(code)
    }

    fn get_callback_state(url: &url::Url) -> anyhow::Result<String> {
        let url_params = url
            .query_pairs()
            .into_owned()
            .collect::<std::collections::HashMap<_, _>>();
        Ok(url_params["state"].to_owned())
    }

    pub fn get_user_info(&self) -> anyhow::Result<crate::openai::OpenAIUserInfo> {
        if let Some(id_token) = self.id_token.clone() {
            let splitted_jwt_strings: Vec<_> = id_token.split('.').collect();
            let jwt_body = splitted_jwt_strings
                .get(1)
                .ok_or(anyhow::anyhow!("Could not find separator in jwt string."))?;
            let decoded_jwt_body = general_purpose::URL_SAFE_NO_PAD.decode(jwt_body)?;
            let converted_jwt_body = String::from_utf8(decoded_jwt_body)?;
            return Ok(serde_json::from_str::<crate::openai::OpenAIUserInfo>(
                &converted_jwt_body,
            )?);
        }
        anyhow::bail!("[OpenAIAuth0] You are not logged in")
    }

    pub async fn authenticate(&mut self) -> anyhow::Result<String> {
        if self.use_cache
            && self.access_token.is_some()
            && self.expires.is_some()
            && self
                .expires
                .context("[OpenAIAuth0] Expires proproties not exist")?
                > chrono::Utc::now()
        {
            return Ok(self
                .access_token
                .clone()
                .context("[OpenAIAuth0] You are not logged in")?);
        }

        if !self.email_regex.is_match(&self.email) || self.password.is_empty() {
            anyhow::bail!("[OpenAIAuth0] Invalid email or password")
        }

        self.login().await
    }

    async fn login(&mut self) -> anyhow::Result<String> {
        let code_verifier = Self::generate_code_verifier();
        let code_challenge = Self::generate_code_challenge(&code_verifier);

        log::debug!("code_verifier: {}", code_verifier);
        log::debug!("code_challenge: {}", code_challenge);

        let url = format!("https://auth0.openai.com/authorize?client_id={}&audience=https%3A%2F%2Fapi.openai.com%2Fv1&redirect_uri=com.openai.chat%3A%2F%2Fauth0.openai.com%2Fios%2Fcom.openai.chat%2Fcallback&scope=openid%20email%20profile%20offline_access%20model.request%20model.read%20organization.read%20offline&response_type=code&code_challenge={}&code_challenge_method=S256&prompt=login", CLIENT_ID, code_challenge);

        let mut headers = self.req_headers.clone();
        headers.insert(
            "Referer",
            HeaderValue::from_str("https://ios.chat.openai.com/")?,
        );
        let resp = self.session.get(url).headers(headers).send().await?;

        if resp.status().is_success() {
            let state = Self::get_callback_state(resp.url())?;
            self.identifier_handler(&code_verifier, &state).await
        } else {
            anyhow::bail!("[OpenAIAuth0] Error request login url")
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
        headers.insert("Referer", HeaderValue::from_str(&url).unwrap());
        headers.insert(
            "Origin",
            HeaderValue::from_str("https://auth0.openai.com").unwrap(),
        );
        let data = serde_json::json!({
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
            log::debug!(
                "[OpenAIAuth0] Redirection request headers{:?}",
                resp.headers()
            );
            let location = resp
                .headers()
                .get("Location")
                .context("[OpenAIAuth0] Error check email")?
                .to_str()?;
            self.login_handler0(code_verifier, state, location, &url)
                .await
        } else {
            anyhow::bail!("[OpenAIAuth0] Error check email")
        }
    }

    async fn login_handler0(
        &mut self,
        code_verifier: &str,
        state: &str,
        location: &str,
        referrer: &str,
    ) -> anyhow::Result<String> {
        let url = format!("https://auth0.openai.com{}", location);
        self.req_headers
            .insert("Referer", HeaderValue::from_str(referrer).unwrap());
        self.req_headers.remove("Origin");
        let data = json!({
            "state": state,
            "username": self.email.to_string(),
            "password": self.password.to_string(),
            "action": "default"
        });
        let resp = self
            .session
            .post(&url)
            .headers(self.req_headers.clone())
            .json(&data)
            .send()
            .await?;

        if resp.status().is_redirection() {
            let location = resp
                .headers()
                .get("Location")
                .context("[OpenAIAuth0] Error login.")?
                .to_str()?;
            if !location.starts_with("/authorize/resume?") {
                anyhow::bail!("[OpenAIAuth0] Login failed")
            }
            self.login_handler1(code_verifier, location, &url).await
        } else if resp.status().is_client_error() {
            anyhow::bail!("[OpenAIAuth0] Wrong email or password")
        } else {
            anyhow::bail!("[OpenAIAuth0] Error login")
        }
    }

    #[async_recursion]
    async fn login_handler1(
        &mut self,
        code_verifier: &str,
        location: &str,
        referrer: &str,
    ) -> anyhow::Result<String> {
        let url = format!("https://auth0.openai.com{}", location);
        let mut headers = self.req_headers.clone();
        headers.insert("Referer", HeaderValue::from_str(referrer).unwrap());
        let resp = self.session.get(&url).headers(headers).send().await?;

        log::debug!(
            "[OpenAIAuth0]-login_handler1 Request headers: {:?}",
            resp.headers()
        );

        if resp.status().is_redirection() {
            let location = resp
                .headers()
                .get("Location")
                .context("[OpenAIAuth0] Error login")?
                .to_str()?;
            if location.starts_with("/u/mfa-otp-challenge?") {
                if self.mfa.is_none() {
                    anyhow::bail!("[OpenAIAuth0] MFA required.")
                }
                return self.login_handler2(code_verifier, location).await;
            } else if !location
                .starts_with("com.openai.chat://auth0.openai.com/ios/com.openai.chat/callback?")
            {
                anyhow::bail!("[OpenAIAuth0] Login callback failed.")
            } else {
                return self.do_get_access_token(code_verifier, location).await;
            }
        }
        anyhow::bail!("[OpenAIAuth0] Error login.")
    }

    #[async_recursion]
    async fn login_handler2(
        &mut self,
        code_verifier: &str,
        location: &str,
    ) -> anyhow::Result<String> {
        let url = format!("https://auth0.openai.com{}", location);
        let state = Self::get_callback_state(&Url::parse(&url)?)?;
        let data = json!({
            "state": state,
            "code": self.mfa.clone().context("[OpenAIAuth0] MFA required.")?,
            "action": "default"
        });

        let mut headers = self.req_headers.clone();
        headers.insert("Referer", HeaderValue::from_str(&url)?);
        headers.insert(
            "Origin",
            HeaderValue::from_static("https://auth0.openai.com"),
        );
        headers.insert("User-Agent", HeaderValue::from_static(UA));

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
                .context("[OpenAIAuth0] Error login.")?
                .to_str()?;

            if location.starts_with("/authorize/resume?") {
                if self.mfa.is_none() {
                    anyhow::bail!("[OpenAIAuth0] MFA failed.")
                }
            }
            return self.login_handler1(code_verifier, location, &url).await;
        }
        if status.is_client_error() {
            anyhow::bail!("[OpenAIAuth0] Wrong MFA code")
        }
        anyhow::bail!("[OpenAIAuth0] Error login")
    }

    async fn do_get_access_token(
        &mut self,
        code_verifier: &str,
        callback_url: &str,
    ) -> anyhow::Result<String> {
        let url = Url::parse(callback_url)?;

        let code = Self::get_callback_code(&url)?;

        let data = json!({
            "redirect_uri": "com.openai.chat://auth0.openai.com/ios/com.openai.chat/callback".to_string(),
            "grant_type": "authorization_code".to_string(),
            "client_id": CLIENT_ID.to_string(),
            "code": code,
            "code_verifier": code_verifier.to_string()
        });

        let resp = self
            .session
            .post("https://auth0.openai.com/oauth/token")
            .headers(self.req_headers.clone())
            .json(&data)
            .send()
            .await?;

        if resp.status().is_success() {
            let result = resp
                .json::<AccessToken>()
                .await
                .context("[OpenAIAuth0] Get access token failed, maybe you need a proxy")?;
            self.expires = Some(
                chrono::Utc::now() + chrono::Duration::seconds(i64::from(result.expires_in))
                    - chrono::Duration::minutes(5),
            );

            let access_token = result.access_token.clone();
            self.access_token = Some(result.access_token);
            self.refresh_token = Some(result.refresh_token);
            self.id_token = Some(result.id_token);
            Ok(access_token)
        } else {
            anyhow::bail!("[OpenAIAuth0] Get access token failed")
        }
    }

    pub async fn do_refresh_token(&mut self) -> anyhow::Result<()> {
        let refresh_token = self
            .refresh_token
            .clone()
            .context("[OpenAIAuth0] You are not logged in")?;
        let data = json!({
            "redirect_uri": "com.openai.chat://auth0.openai.com/ios/com.openai.chat/callback".to_string(),
            "grant_type": "refresh_token".to_string(),
            "client_id": CLIENT_ID.to_string(),
            "refresh_token": refresh_token
        });

        let resp = self
            .session
            .post("https://auth0.openai.com/oauth/token")
            .json(&data)
            .send()
            .await?;
        if resp.status().is_success() {
            let result = resp
                .json::<RefreshToken>()
                .await
                .context("[OpenAIAuth0] Get access token failed, maybe you need a proxy")?;
            self.expires = Some(
                chrono::Utc::now() + chrono::Duration::seconds(i64::from(result.expires_in))
                    - chrono::Duration::minutes(5),
            );

            self.access_token = Some(result.access_token);
            self.id_token = Some(result.id_token);
        } else {
            anyhow::bail!("[OpenAIAuth0] Get refresh access token failed")
        }
        Ok(())
    }
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct AccessToken {
    access_token: String,
    refresh_token: String,
    id_token: String,
    expires_in: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct RefreshToken {
    access_token: String,
    id_token: String,
    expires_in: u32,
}
