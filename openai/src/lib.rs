pub mod chatgpt;
pub mod oauth;
pub mod token;

use serde::Deserialize;

pub const DEFAULT_TOKEN_FILE: &str = ".opengpt-access_tokens";
pub type OAuthResult<T, E = anyhow::Error> = anyhow::Result<T, E>;
pub type TokenResult<T, E = anyhow::Error> = anyhow::Result<T, E>;

#[derive(thiserror::Error, Deserialize, Debug)]
pub enum OAuthError {
    #[error("fnvalid request (error {error:?}, error_description {error_description:?})")]
    BadRequest {
        error: String,
        error_description: String,
    },
    #[error("failed to get public key")]
    FailedPubKeyRequest,
    #[error("failed login")]
    FailedLogin,
    #[error("failed logged in")]
    FailedLoginIn,
    #[error("failed get code from callback url")]
    FailedCallbackCode,
    #[error("failed callback url")]
    FailedCallbackURL,
    #[error("invalid request login url")]
    InvalidLoginUrl,
    #[error("invalid email or password")]
    InvalidEmailOrPassword,
    #[error("invalid email")]
    InvalidEmail,
    #[error("invalid Location")]
    InvalidLocation,
    #[error("invalid token")]
    InvalidAccessToken,
    #[error("token expired")]
    TokenExpired,
    #[error("Invalid MFA code")]
    InvalidMFACode,
    #[error("MFA failed")]
    MFAFailed,
    #[error("MFA required")]
    MFARequired,
}

#[derive(thiserror::Error, Debug)]
pub enum TokenStoreError {
    #[error("failed to access token")]
    AccessError,
    #[error("token not found error")]
    NotFoundError,
    #[error("failed token deserialize")]
    DeserializeError,
    #[error("failed to verify access_token")]
    AccessTokenVerifyError,
    #[error("failed to create default token store file")]
    CreateDefaultTokenFileError,
}
