use crate::auth::error::AuthError;
use crate::auth::provide::{AuthenticateData, GrantType};
use crate::auth::AuthClient;
use crate::auth::{
    model::{self, AuthStrategy},
    OPENAI_OAUTH_REVOKE_URL, OPENAI_OAUTH_TOKEN_URL, OPENAI_OAUTH_URL,
};
use crate::warn;
use async_recursion::async_recursion;
use axum::http::HeaderValue;
use reqwest::Client;
use url::Url;

use super::{
    AuthProvider, AuthResult, AuthenticateMfaData, AuthorizationCodeData, IdentifierData,
    RefreshTokenData, RequestContext, RequestContextExt, RevokeTokenData,
};

const PLATFORM_CLIENT_ID: &str = "DRivsnm2Mu42T3KOpqdtwB3NYviHYzwD";
const OPENAI_OAUTH_PLATFORM_CALLBACK_URL: &str = "https://platform.openai.com/auth/callback";

pub(crate) struct PlatformAuthProvider(pub(crate) Client);

impl PlatformAuthProvider {
    pub fn new(inner: Client) -> impl AuthProvider + Send + Sync {
        Self(inner)
    }

    async fn authorize(&self, ctx: &mut RequestContext<'_>) -> AuthResult<()> {
        // Build url
        let url = format!("{OPENAI_OAUTH_URL}/authorize?client_id={PLATFORM_CLIENT_ID}&scope=openid%20email%20profile%20offline_access%20model.request%20model.read%20organization.read%20organization.write&audience=https://api.openai.com/v1&redirect_uri=https://platform.openai.com/auth/callback&response_type=code");

        let resp = self
            .0
            .get(&url)
            .send()
            .await
            .map_err(AuthError::FailedRequest)?
            .ext_context(ctx);

        // Get location path from response headers
        let location = AuthClient::get_location_path(resp.headers())?;

        let resp = self
            .0
            .get(format!("{OPENAI_OAUTH_URL}{location}"))
            .ext_context(ctx)
            .send()
            .await
            .map_err(AuthError::FailedRequest)?
            .ext_context(ctx);

        // Get state from response url
        let state = AuthClient::get_callback_state(&resp.url())?;

        // Set state
        ctx.set_state(state.as_str());

        AuthClient::response_handle_unit(resp).await
    }

    async fn authenticate_username(&self, ctx: &mut RequestContext<'_>) -> AuthResult<()> {
        let url = format!("{OPENAI_OAUTH_URL}/u/login/identifier?state={}", ctx.state);
        let resp = self
            .0
            .post(&url)
            .ext_context(ctx)
            .json(
                &IdentifierData::builder()
                    .action("default")
                    .state(&ctx.state)
                    .username(&ctx.account.username)
                    .js_available(true)
                    .webauthn_available(true)
                    .is_brave(false)
                    .webauthn_platform_available(false)
                    .build(),
            )
            .send()
            .await?
            .ext_context(ctx);

        AuthClient::response_handle_unit(resp)
            .await
            .map_err(|_| (AuthError::InvalidEmail))
    }

    async fn authenticate_password(
        &self,
        ctx: &mut RequestContext<'_>,
    ) -> AuthResult<model::AccessToken> {
        ctx.load_arkose_token().await?;
        let resp = self
            .0
            .post(format!(
                "{OPENAI_OAUTH_URL}/u/login/password?state={}",
                ctx.state
            ))
            .ext_context(ctx)
            .json(
                &AuthenticateData::builder()
                    .action("default")
                    .state(&ctx.state)
                    .username(&ctx.account.username)
                    .password(&ctx.account.password)
                    .build(),
            )
            .send()
            .await
            .map_err(AuthError::FailedRequest)?
            .ext_context(ctx);

        // If resp status is client error return InvalidEmailOrPassword
        if resp.status().is_client_error() {
            return Err(AuthError::InvalidEmailOrPassword);
        }

        // Get location path
        let location = AuthClient::get_location_path(&resp.headers())?;

        // If location path starts with https://chat.openai.com/, return invalid location path
        if location.contains("https://chat.openai.com/") {
            warn!("PlatformAuthProvider::authenticate_password: invalid location path: {location}");
            return Err(AuthError::InvalidLocationPath);
        }

        // If location path starts with /authorize/resume?
        if location.starts_with("/authorize/resume?") {
            return self.authenticate_resume(ctx, location).await;
        }

        Err(AuthError::FailedLogin)
    }

    async fn authenticate_resume(
        &self,
        ctx: &mut RequestContext<'_>,
        location: &str,
    ) -> AuthResult<model::AccessToken> {
        let resp = self
            .0
            .get(&format!("{OPENAI_OAUTH_URL}{location}"))
            .ext_context(ctx)
            .send()
            .await
            .map_err(AuthError::FailedRequest)?
            .ext_context(ctx);

        // maybe auth failed
        let _ = AuthClient::check_auth_callback_state(resp.url())?;

        // Get location path
        let location: &str = AuthClient::get_location_path(&resp.headers())?;

        // If location path starts with /u/mfa-otp-challenge?
        if location.starts_with("/u/mfa-otp-challenge?") {
            return self.authenticate_mfa(ctx, location).await;
        }

        // If location path starts with https://platform.openai.com/auth/callback
        if location.starts_with(OPENAI_OAUTH_PLATFORM_CALLBACK_URL) {
            return self.authorization_code(location).await;
        }

        Err(AuthError::FailedCallbackURL)
    }

    #[async_recursion]
    async fn authenticate_mfa(
        &self,
        ctx: &mut RequestContext<'_>,
        location: &str,
    ) -> AuthResult<model::AccessToken> {
        // Get mfa code
        let mfa_code = &ctx.account.mfa.clone().ok_or(AuthError::MFARequired)?;

        // Parse url
        let url = Url::parse(&format!("{OPENAI_OAUTH_URL}{}", location))
            .map_err(AuthError::InvalidLoginUrl)?;

        // Get state from url
        let state = AuthClient::get_callback_state(&url)?;

        let data = AuthenticateMfaData::builder()
            .action("default")
            .state(&state)
            .code(mfa_code)
            .build();

        let resp = self
            .0
            .post(url)
            .ext_context(ctx)
            .json(&data)
            .header(
                reqwest::header::REFERER,
                HeaderValue::from_static(OPENAI_OAUTH_URL),
            )
            .header(
                reqwest::header::ORIGIN,
                HeaderValue::from_static(OPENAI_OAUTH_URL),
            )
            .send()
            .await
            .map_err(AuthError::FailedRequest)?
            .ext_context(ctx);

        let location: &str = AuthClient::get_location_path(&resp.headers())?;

        // If location starts with /authorize/resume? and mfa is none, return mfa failed
        if location.starts_with("/authorize/resume?") && ctx.account.mfa.is_none() {
            return Err(AuthError::MFAFailed);
        }
        self.authenticate_resume(ctx, location).await
    }

    async fn authorization_code(&self, location: &str) -> AuthResult<model::AccessToken> {
        // Parse url
        let url = Url::parse(location).map_err(AuthError::InvalidLoginUrl)?;

        // Get code from url
        let code = AuthClient::get_callback_code(&url)?;

        let data = AuthorizationCodeData::builder()
            .redirect_uri(OPENAI_OAUTH_PLATFORM_CALLBACK_URL)
            .grant_type(GrantType::AuthorizationCode)
            .client_id(PLATFORM_CLIENT_ID)
            .code(&code)
            .code_verifier(None)
            .build();

        let resp = self
            .0
            .post(OPENAI_OAUTH_TOKEN_URL)
            .json(&data)
            .send()
            .await
            .map_err(AuthError::FailedRequest)?;

        let access_token = AuthClient::response_handle::<model::OAuthAccessToken>(resp).await?;
        Ok(model::AccessToken::OAuth(access_token))
    }
}

#[async_trait::async_trait]
impl AuthProvider for PlatformAuthProvider {
    fn supports(&self, t: &AuthStrategy) -> bool {
        t.eq(&AuthStrategy::Platform)
    }

    async fn do_access_token(
        &self,
        account: &model::AuthAccount,
    ) -> AuthResult<model::AccessToken> {
        let mut ctx = RequestContext::new(account);
        // authorized
        self.authorize(&mut ctx).await?;

        // check username
        self.authenticate_username(&mut ctx).await?;

        // check password and username
        self.authenticate_password(&mut ctx).await
    }

    async fn do_refresh_token(&self, refresh_token: &str) -> AuthResult<model::RefreshToken> {
        let refresh_token = AuthClient::trim_bearer(refresh_token)?;
        let data = RefreshTokenData::builder()
            .redirect_uri(OPENAI_OAUTH_PLATFORM_CALLBACK_URL)
            .grant_type(GrantType::RefreshToken)
            .client_id(PLATFORM_CLIENT_ID)
            .refresh_token(refresh_token)
            .build();

        let resp = self
            .0
            .post(OPENAI_OAUTH_TOKEN_URL)
            .json(&data)
            .send()
            .await?;

        let mut token = AuthClient::response_handle::<model::RefreshToken>(resp).await?;
        token.refresh_token = refresh_token.to_owned();
        Ok(token)
    }

    async fn do_revoke_token(&self, refresh_token: &str) -> AuthResult<()> {
        let refresh_token = AuthClient::trim_bearer(refresh_token)?;
        let data = RevokeTokenData::builder()
            .client_id(PLATFORM_CLIENT_ID)
            .token(refresh_token)
            .build();

        let resp = self
            .0
            .post(OPENAI_OAUTH_REVOKE_URL)
            .json(&data)
            .send()
            .await
            .map_err(AuthError::FailedRequest)?;

        AuthClient::response_handle_unit(resp).await
    }
}
