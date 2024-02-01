pub mod error;
pub mod model;
pub mod provide;

extern crate regex;

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;

use regex::Regex;
use reqwest::dns::Resolve;
use reqwest::header::{self, HeaderMap, HeaderValue};
use reqwest::impersonate::Impersonate;
use reqwest::redirect::Policy;
use serde::de::DeserializeOwned;

use base64::{engine::general_purpose, Engine as _};
use rand::Rng;
use reqwest::{Client, Proxy, StatusCode, Url};
use sha2::{Digest, Sha256};
use tokio::sync::OnceCell;

use crate::constant::API_AUTH_SESSION_COOKIE_KEY;
use crate::debug;
use crate::URL_CHATGPT_API;
use error::AuthError;

use self::model::{ApiKeyData, AuthStrategy};
#[cfg(feature = "preauth")]
use self::provide::apple::AppleAuthProvider;
use self::provide::platform::PlatformAuthProvider;
use self::provide::web::WebAuthProvider;
use self::provide::{AuthProvider, AuthResult};

const OPENAI_API_URL: &str = "https://api.openai.com";
const OPENAI_OAUTH_URL: &str = "https://auth0.openai.com";
const OPENAI_OAUTH_TOKEN_URL: &str = "https://auth0.openai.com/oauth/token";
const OPENAI_OAUTH_REVOKE_URL: &str = "https://auth0.openai.com/oauth/revoke";

static EMAIL_REGEX: OnceCell<Regex> = OnceCell::const_new();

/// You do **not** have to wrap the `Client` in an [`Rc`] or [`Arc`] to **reuse** it,
/// because it already uses an [`Arc`] internally.
///
/// [`Rc`]: std::rc::Rc
#[derive(Clone)]
pub struct AuthClient {
    inner: Client,
    providers: Arc<Vec<Box<dyn AuthProvider + Send + Sync>>>,
}

impl AuthClient {
    pub async fn refresh_session(&self, session: &str) -> AuthResult<model::AccessToken> {
        let resp = self
            .inner
            .get(format!("{URL_CHATGPT_API}/api/auth/session"))
            .header(
                header::COOKIE,
                format!("{API_AUTH_SESSION_COOKIE_KEY}={session};"),
            )
            .send()
            .await
            .map_err(AuthError::FailedRequest)?;

        match resp.error_for_status_ref() {
            Ok(_) => Self::exstract_session_hanlder(resp).await,
            Err(err) => Err(Self::handle_error(resp, err).await),
        }
    }

    pub async fn dashboard_login(&self, access_token: &str) -> AuthResult<model::DashSession> {
        let access_token = access_token.replace("Bearer ", "");
        let resp = self
            .inner
            .post(format!("{OPENAI_API_URL}/dashboard/onboarding/login"))
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(AuthError::FailedRequest)?;

        Self::response_handle(resp).await
    }

    pub async fn api_key_list(&self, sensitive_id: &str) -> AuthResult<model::ApiKeyList> {
        let resp = self
            .inner
            .get(format!("{OPENAI_API_URL}/dashboard/user/api_keys"))
            .bearer_auth(sensitive_id)
            .send()
            .await
            .map_err(AuthError::FailedRequest)?;
        Self::response_handle(resp).await
    }

    pub async fn api_key<'a>(
        &self,
        sensitive_id: &str,
        data: ApiKeyData<'a>,
    ) -> AuthResult<model::ApiKey> {
        let resp = self
            .inner
            .post(format!("{OPENAI_API_URL}/dashboard/user/api_keys"))
            .bearer_auth(sensitive_id)
            .json(&data)
            .send()
            .await
            .map_err(AuthError::FailedRequest)?;
        Self::response_handle(resp).await
    }

    pub async fn billing_credit_grants(&self, sensitive_id: &str) -> AuthResult<model::Billing> {
        let resp = self
            .inner
            .get(format!("{OPENAI_API_URL}/dashboard/billing/credit_grants"))
            .bearer_auth(sensitive_id)
            .send()
            .await
            .map_err(AuthError::FailedRequest)?;
        Self::response_handle(resp).await
    }

    async fn exstract_session_hanlder(resp: reqwest::Response) -> AuthResult<model::AccessToken> {
        let session = resp
            .cookies()
            .find(|c| c.name().eq(API_AUTH_SESSION_COOKIE_KEY))
            .map(|c| model::Session {
                value: c.value().to_owned(),
                expires: c.expires(),
            });

        match session {
            Some(session) => {
                let mut session_access_token = resp
                    .json::<model::SessionAccessToken>()
                    .await
                    .map_err(AuthError::DeserializeError)?;
                session_access_token.session_token = Some(session);
                Ok(model::AccessToken::Session(session_access_token))
            }
            None => Err(AuthError::FailedAccessToken(resp.text().await?)),
        }
    }

    async fn response_handle<U: DeserializeOwned>(resp: reqwest::Response) -> AuthResult<U> {
        match resp.error_for_status_ref() {
            Ok(_) => {
                let result = resp
                    .json::<U>()
                    .await
                    .map_err(AuthError::DeserializeError)?;
                Ok(result)
            }
            Err(err) => Err(Self::handle_error(resp, err).await),
        }
    }

    async fn response_handle_unit(resp: reqwest::Response) -> AuthResult<()> {
        match resp.error_for_status_ref() {
            Ok(_) => Ok(()),
            Err(err) => Err(Self::handle_error(resp, err).await),
        }
    }

    async fn handle_error(resp: reqwest::Response, err: reqwest::Error) -> AuthError {
        match err.status() {
            Some(status_code) => {
                // Extract the error message from the response
                let msg = resp.text().await.unwrap_or_default();
                match status_code {
                    StatusCode::UNAUTHORIZED => AuthError::Unauthorized(msg),
                    StatusCode::TOO_MANY_REQUESTS => AuthError::TooManyRequests(msg),
                    StatusCode::BAD_REQUEST => AuthError::BadRequest(msg),
                    StatusCode::FORBIDDEN => AuthError::Forbidden(msg),
                    _ => AuthError::ServerError(err),
                }
            }
            _ => AuthError::ServerError(err),
        }
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

    /// Check the state of the auth callback
    /// You do not have an account because it has been deleted or deactivated.
    /// If you believe this was an error, please contact us through our help center at help.openai.com. (error=account_deactivated)
    fn check_auth_callback_state(url: &Url) -> AuthResult<HashMap<String, Vec<String>>> {
        let mut url_params = HashMap::new();
        url.query_pairs().into_owned().for_each(|(key, value)| {
            url_params
                .entry(key)
                .and_modify(|v: &mut Vec<String>| v.push(value.clone()))
                .or_insert(vec![value]);
        });

        // If the user denies the request, the URL will contain an error parameter
        if let Some(error) = url_params.get("error") {
            return if let Some(error_description) = url_params.get("error_description") {
                let msg = error_description.join(",");
                Err(AuthError::InvalidLogin(msg))
            } else {
                Err(AuthError::InvalidLogin(error.join(",")))
            };
        }

        Ok(url_params)
    }

    /// Get the callback code from the url
    fn get_callback_code(url: &Url) -> AuthResult<String> {
        // Return the code if it exists
        let callback_code = Self::check_auth_callback_state(&url)?
            .get("code")
            .map(|c| c.first())
            .flatten()
            .ok_or(AuthError::FailedCallbackCode)?
            .to_string();
        Ok(callback_code)
    }

    /// Get the callback state from the url
    fn get_callback_state(url: &Url) -> AuthResult<String> {
        let url_params = url.query_pairs().into_owned().collect::<HashMap<_, _>>();
        debug!("get_callback_state: {:?}", url_params);
        let state = url_params.get("state").ok_or(AuthError::FailedState)?;
        Ok(state.to_owned())
    }

    /// Get the location path from the header
    fn get_location_path(header: &HeaderMap<HeaderValue>) -> AuthResult<&str> {
        debug!("get_location_path: {:?}", header);
        let location = header
            .get("Location")
            .ok_or(AuthError::InvalidLocation)?
            .to_str()
            .map_err(|_| AuthError::InvalidLocation)?;
        Ok(location)
    }

    /// Trim the bearer token from the refresh token
    fn trim_bearer(t: &str) -> AuthResult<&str> {
        let refresh_token = t.trim_start_matches("Bearer ");
        if refresh_token.is_empty() {
            return Err(AuthError::InvalidRefreshToken);
        }
        Ok(refresh_token)
    }
}

#[async_trait::async_trait]
impl AuthProvider for AuthClient {
    fn supports(&self, t: &AuthStrategy) -> bool {
        self.providers.iter().any(|strategy| strategy.supports(t))
    }

    async fn do_access_token(
        &self,
        account: &model::AuthAccount,
    ) -> AuthResult<model::AccessToken> {
        let regex = EMAIL_REGEX
            .get_or_try_init(|| async {
                Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,7}\b")
            })
            .await
            .map_err(AuthError::InvalidRegex)?;

        if !regex.is_match(&account.username) || account.password.is_empty() {
            return Err(AuthError::InvalidEmailOrPassword);
        }

        // Try supported providers
        for provider in self.providers.iter() {
            if provider.supports(&account.option) {
                return provider.do_access_token(account).await;
            }
        }

        Err(AuthError::NotSupportedImplementation)
    }

    async fn do_revoke_token(&self, refresh_token: &str) -> AuthResult<()> {
        let mut result: Option<AuthResult<()>> = None;
        for handle in self.providers.iter() {
            if handle.supports(&AuthStrategy::Apple) || handle.supports(&AuthStrategy::Platform) {
                let res = handle.do_revoke_token(refresh_token).await;
                match res {
                    Ok(ok) => {
                        result = Some(Ok(ok));
                        break;
                    }
                    Err(err) => {
                        result = Some(Err(err));
                    }
                }
            }
        }

        result.ok_or(AuthError::NotSupportedImplementation)?
    }

    async fn do_refresh_token(&self, refresh_token: &str) -> AuthResult<model::RefreshToken> {
        let mut result: Option<AuthResult<model::RefreshToken>> = None;

        for handle in self.providers.iter() {
            if handle.supports(&AuthStrategy::Apple) || handle.supports(&AuthStrategy::Platform) {
                let res = handle.do_refresh_token(refresh_token).await;
                match res {
                    Ok(ok) => {
                        result = Some(Ok(ok));
                        break;
                    }
                    Err(err) => {
                        result = Some(Err(err));
                    }
                }
            }
        }

        result.ok_or(AuthError::NotSupportedImplementation)?
    }
}

pub struct AuthClientBuilder {
    inner: reqwest::ClientBuilder,
}

impl AuthClientBuilder {
    // Proxy options
    pub fn proxy(mut self, proxy: Option<Url>) -> Self {
        if let Some(url) = proxy {
            self.inner = self
                .inner
                .proxy(Proxy::all(url).expect("reqwest: invalid proxy url"));
        } else {
            self.inner = self.inner.no_proxy();
        }
        self
    }

    // Timeout options

    /// Enables a request timeout.
    ///
    /// The timeout is applied from when the request starts connecting until the
    /// response body has finished.
    ///
    /// Default is no timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    /// Set a timeout for only the connect phase of a `Client`.
    ///
    /// Default is `None`.
    ///
    /// # Note
    ///
    /// This **requires** the futures be executed in a tokio runtime with
    /// a tokio timer enabled.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.connect_timeout(timeout);
        self
    }

    // HTTP options

    /// Set an optional timeout for idle sockets being kept-alive.
    ///
    /// Pass `None` to disable timeout.
    ///
    /// Default is 90 seconds.
    pub fn pool_idle_timeout<D>(mut self, val: D) -> Self
    where
        D: Into<Option<Duration>>,
    {
        self.inner = self.inner.pool_idle_timeout(val);
        self
    }

    /// Sets the maximum idle connection per host allowed in the pool.
    pub fn pool_max_idle_per_host(mut self, max: usize) -> Self {
        self.inner = self.inner.pool_max_idle_per_host(max);
        self
    }

    /// Set that all sockets have `SO_KEEPALIVE` set with the supplied duration.
    ///
    /// If `None`, the option will not be set.
    pub fn tcp_keepalive<D>(mut self, val: D) -> Self
    where
        D: Into<Option<Duration>>,
    {
        self.inner = self.inner.tcp_keepalive(val);
        self
    }

    /// Sets the necessary values to mimic the specified impersonate client version.
    pub fn impersonate(mut self, ver: Impersonate) -> Self {
        self.inner = self.inner.impersonate(ver);
        self
    }

    /// Sets the `User-Agent` header to be used by this client.
    pub fn user_agent(mut self, value: &str) -> Self {
        self.inner = self.inner.user_agent(value);
        self
    }

    /// Bind to a local IP Address.
    pub fn local_address<T>(mut self, addr: T) -> Self
    where
        T: Into<Option<IpAddr>>,
    {
        self.inner = self.inner.local_address(addr);
        self
    }

    /// Set that all sockets are bound to the configured IPv4 or IPv6 address (depending on host's
    /// preferences) before connection.
    pub fn local_addresses(mut self, addr_ipv4: Ipv4Addr, addr_ipv6: Ipv6Addr) -> Self {
        self.inner = self.inner.local_addresses(addr_ipv4, addr_ipv6);
        self
    }

    /// Override the DNS resolver implementation.
    ///
    /// Pass an `Arc` wrapping a trait object implementing `Resolve`.
    /// Overrides for specific names passed to `resolve` and `resolve_to_addrs` will
    /// still be applied on top of this resolver.
    pub fn dns_resolver<R: Resolve + 'static>(mut self, resolver: Arc<R>) -> Self {
        self.inner = self.inner.dns_resolver(resolver);
        self
    }

    /// Controls the use of certificate validation.
    pub fn danger_accept_invalid_certs(mut self, enable: bool) -> Self {
        self.inner = self.inner.danger_accept_invalid_certs(enable);
        self
    }

    /// Enable Encrypted Client Hello (Secure SNI)
    pub fn enable_ech_grease(mut self, enable: bool) -> Self {
        self.inner = self.inner.enable_ech_grease(enable);
        self
    }

    /// Enable TLS permute_extensions
    pub fn permute_extensions(mut self, enable: bool) -> Self {
        self.inner = self.inner.permute_extensions(enable);
        self
    }

    pub fn build(self) -> AuthClient {
        let client = self
            .inner
            .default_headers({
                let mut headers = HeaderMap::new();
                headers.insert(header::ORIGIN, HeaderValue::from_static(OPENAI_OAUTH_URL));
                headers.insert(header::REFERER, HeaderValue::from_static(OPENAI_OAUTH_URL));
                headers
            })
            .build()
            .expect("ClientBuilder::build()");

        let mut providers: Vec<Box<dyn AuthProvider + Send + Sync>> = Vec::with_capacity(3);
        providers.push(Box::new(WebAuthProvider::new(client.clone())));
        providers.push(Box::new(PlatformAuthProvider::new(client.clone())));
        #[cfg(feature = "preauth")]
        providers.push(Box::new(AppleAuthProvider::new(client.clone())));

        AuthClient {
            inner: client,
            providers: Arc::new(providers),
        }
    }

    pub fn builder() -> AuthClientBuilder {
        AuthClientBuilder {
            inner: Client::builder().redirect(Policy::none()),
        }
    }
}
