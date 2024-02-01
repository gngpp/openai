use crate::auth::error::AuthError;
use crate::auth::provide::{AuthenticateData, GrantType};
use crate::auth::AuthClient;
use crate::auth::{
    model::{self, AuthStrategy},
    OPENAI_OAUTH_REVOKE_URL, OPENAI_OAUTH_TOKEN_URL, OPENAI_OAUTH_URL,
};
use crate::{warn, with_context};
use async_recursion::async_recursion;
use axum::http::HeaderValue;
use reqwest::Client;
use url::Url;

use super::{
    AuthProvider, AuthResult, AuthenticateMfaData, AuthorizationCodeData, IdentifierData,
    RefreshTokenData, RequestContext, RequestContextExt, RevokeTokenData,
};

const STATE: &str = "TMf_R7zSeBRzTs86WAfQJh9Q_AbDh3382e7Y-pae1wQ";
const APP_VERSION: &str = "7657";
const AUTH0_CLIENT: &str = "eyJlbnYiOnsiaU9TIjoiMTYuNSIsInN3aWZ0IjoiNS54In0sInZlcnNpb24iOiIyLjUuMCIsIm5hbWUiOiJBdXRoMC5zd2lmdCJ9";
const APPLE_CLIENT_ID: &str = "pdlLIX2Y72MIl2rhLhTE9VV9bN905kBh";
const OPENAI_OAUTH_APPLE_CALLBACK_URL: &str =
    "com.openai.chat://auth0.openai.com/ios/com.openai.chat/callback";

pub(crate) struct PreAuthProvider;

impl PreAuthProvider {
    fn get_preauth_cookie(&self) -> AuthResult<String> {
        with_context!(pop_preauth_cookie).ok_or(AuthError::PreauthCookieNotFound)
    }
}

pub(crate) struct AppleAuthProvider {
    inner: Client,
    preauth_provider: PreAuthProvider,
}

impl AppleAuthProvider {
    pub fn new(inner: Client) -> impl AuthProvider + Send + Sync {
        Self {
            inner,
            preauth_provider: PreAuthProvider,
        }
    }

    async fn authorize(&self, ctx: &mut RequestContext<'_>) -> AuthResult<()> {
        // Get the preauth cookie.
        let preauth_cookie = self.preauth_provider.get_preauth_cookie()?;

        // Build the URL.
        let code_challenge = ctx.code_challenge.as_str();
        let url = format!("{OPENAI_OAUTH_URL}/authorize?state={STATE}&ios_app_version={APP_VERSION}&client_id={APPLE_CLIENT_ID}&redirect_uri={OPENAI_OAUTH_APPLE_CALLBACK_URL}&code_challenge={code_challenge}&scope=openid%20email%20profile%20offline_access%20model.request%20model.read%20organization.read%20organization.write&prompt=login&preauth_cookie={preauth_cookie}&audience=https://api.openai.com/v1&code_challenge_method=S256&response_type=code&auth0Client={AUTH0_CLIENT}");

        let resp = self
            .inner
            .get(url)
            .header(
                reqwest::header::REFERER,
                HeaderValue::from_static(OPENAI_OAUTH_URL),
            )
            .send()
            .await
            .map_err(AuthError::FailedRequest)?
            .ext_context(ctx);

        // Get the location path from the response headers.
        let location = AuthClient::get_location_path(resp.headers())?;

        let resp = self
            .inner
            .get(format!("{OPENAI_OAUTH_URL}{location}"))
            .ext_context(ctx)
            .send()
            .await
            .map_err(AuthError::FailedRequest)?
            .ext_context(ctx);

        // Get the callback state from the URL.
        let state = AuthClient::get_callback_state(resp.url())?;

        // Set the callback state.
        ctx.set_state(state.as_str());

        AuthClient::response_handle_unit(resp).await
    }

    async fn authenticate_username(&self, ctx: &mut RequestContext<'_>) -> AuthResult<()> {
        let url = format!("{OPENAI_OAUTH_URL}/u/login/identifier?state={}", ctx.state);
        let resp = self
            .inner
            .post(&url)
            .ext_context(ctx)
            .form(
                &IdentifierData::builder()
                    .action("default")
                    .state(&ctx.state)
                    .username(&ctx.account.username)
                    .js_available(true)
                    .webauthn_available(true)
                    .is_brave(false)
                    .webauthn_platform_available(true)
                    .build(),
            )
            .send()
            .await
            .map_err(AuthError::FailedRequest)?
            .ext_context(ctx);

        AuthClient::response_handle_unit(resp)
            .await
            .map_err(|_| AuthError::InvalidEmail)
    }

    async fn authenticate_password(
        &self,
        ctx: &mut RequestContext<'_>,
    ) -> AuthResult<model::AccessToken> {
        ctx.load_arkose_token().await?;

        let resp = self
            .inner
            .post(format!(
                "{OPENAI_OAUTH_URL}/u/login/password?state={}",
                ctx.state
            ))
            .ext_context(ctx)
            .form(
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

        // Get the location path from the response headers.
        let location = AuthClient::get_location_path(&resp.headers())?;

        // If the location contains "https://chat.openai.com/", it means that the login failed.
        if location.contains("https://chat.openai.com/") {
            warn!("AppleAuthProvider::authenticate_password: location contains {location}");
            return Err(AuthError::InvalidLocationPath);
        }

        // If the location contains "/authorize/resume?", it means that the login was successful.
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
            .inner
            .get(&format!("{OPENAI_OAUTH_URL}{location}"))
            .ext_context(ctx)
            .send()
            .await
            .map_err(AuthError::FailedRequest)?
            .ext_context(ctx);

        // If resp status is client error return InvalidEmailOrPassword
        if resp.status().is_client_error() {
            return Err(AuthError::InvalidEmailOrPassword);
        }

        // maybe auth failed
        let _ = AuthClient::check_auth_callback_state(resp.url())?;

        // If get_location_path returns an error, it means that the location is invalid.
        let location: &str = AuthClient::get_location_path(&resp.headers())?;
        if location.starts_with("/u/mfa-otp-challenge?") {
            // If the location contains "/u/mfa-otp-challenge?", it means that MFA is required.
            let mfa_code = ctx.account.mfa.clone().ok_or(AuthError::MFARequired)?;
            return self.authenticate_mfa(ctx, &mfa_code, location).await;
        }

        // Indicates successful login.
        if location.starts_with(OPENAI_OAUTH_APPLE_CALLBACK_URL) {
            return self.authorization_code(ctx, location).await;
        }

        // Return an error if the location is invalid.
        Err(AuthError::FailedCallbackURL)
    }

    #[async_recursion]
    async fn authenticate_mfa(
        &self,
        ctx: &mut RequestContext<'_>,
        mfa_code: &str,
        location: &str,
    ) -> AuthResult<model::AccessToken> {
        // Concat the location with the base URL.
        let url = Url::parse(&format!("{OPENAI_OAUTH_URL}{}", location))
            .map_err(AuthError::InvalidLoginUrl)?;

        // Get the callback state from the URL.
        let state = AuthClient::get_callback_state(&url)?;

        let resp = self
            .inner
            .post(url)
            .json(
                &AuthenticateMfaData::builder()
                    .action("default")
                    .state(&state)
                    .code(mfa_code)
                    .build(),
            )
            .ext_context(ctx)
            .send()
            .await
            .map_err(AuthError::FailedRequest)?
            .ext_context(ctx);

        let location: &str = AuthClient::get_location_path(&resp.headers())?;

        // If the location contains "/authorize/resume?", it means that the login was successful.
        if location.starts_with("/authorize/resume?") && ctx.account.mfa.is_none() {
            return Err(AuthError::MFAFailed);
        }

        self.authenticate_resume(ctx, location).await
    }

    async fn authorization_code(
        &self,
        ctx: &mut RequestContext<'_>,
        location: &str,
    ) -> AuthResult<model::AccessToken> {
        // Parse the URL.
        let url = Url::parse(location).map_err(AuthError::InvalidLoginUrl)?;

        // Get the callback code from the URL.
        let code = AuthClient::get_callback_code(&url)?;

        let resp = self
            .inner
            .post(OPENAI_OAUTH_TOKEN_URL)
            .ext_context(ctx)
            .json(
                &AuthorizationCodeData::builder()
                    .redirect_uri(OPENAI_OAUTH_APPLE_CALLBACK_URL)
                    .grant_type(GrantType::AuthorizationCode)
                    .client_id(APPLE_CLIENT_ID)
                    .code(&code)
                    .code_verifier(Some(&ctx.code_verifier))
                    .build(),
            )
            .send()
            .await
            .map_err(AuthError::FailedRequest)?;

        // If the response contains "error", it means that the login failed.
        let access_token = AuthClient::response_handle::<model::OAuthAccessToken>(resp).await?;

        Ok(model::AccessToken::OAuth(access_token))
    }
}

#[async_trait::async_trait]
impl AuthProvider for AppleAuthProvider {
    fn supports(&self, t: &AuthStrategy) -> bool {
        t.eq(&AuthStrategy::Apple)
    }

    async fn do_access_token(
        &self,
        account: &model::AuthAccount,
    ) -> AuthResult<model::AccessToken> {
        let code_verifier = AuthClient::generate_code_verifier();
        let code_challenge = AuthClient::generate_code_challenge(&code_verifier);

        let mut ctx = RequestContext::new(account);
        ctx.set_code_verifier(code_verifier);
        ctx.set_code_challenge(code_challenge);

        // authorize
        self.authorize(&mut ctx).await?;

        // check username
        self.authenticate_username(&mut ctx).await?;

        // check password and username
        self.authenticate_password(&mut ctx).await
    }

    async fn do_refresh_token(&self, refresh_token: &str) -> AuthResult<model::RefreshToken> {
        let refresh_token = AuthClient::trim_bearer(refresh_token)?;
        let data = RefreshTokenData::builder()
            .redirect_uri(OPENAI_OAUTH_APPLE_CALLBACK_URL)
            .grant_type(GrantType::RefreshToken)
            .client_id(APPLE_CLIENT_ID)
            .refresh_token(refresh_token)
            .build();

        let resp = self
            .inner
            .post(OPENAI_OAUTH_TOKEN_URL)
            .json(&data)
            .send()
            .await?;

        let mut token = AuthClient::response_handle::<model::RefreshToken>(resp).await?;
        // Set the refresh token.
        token.refresh_token = refresh_token.to_owned();

        Ok(token)
    }

    async fn do_revoke_token(&self, refresh_token: &str) -> AuthResult<()> {
        let refresh_token = AuthClient::trim_bearer(refresh_token)?;
        let data = RevokeTokenData::builder()
            .client_id(APPLE_CLIENT_ID)
            .token(refresh_token)
            .build();

        let resp = self
            .inner
            .post(OPENAI_OAUTH_REVOKE_URL)
            .json(&data)
            .send()
            .await
            .map_err(AuthError::FailedRequest)?;

        AuthClient::response_handle_unit(resp).await
    }
}
