pub mod chatgpt;
pub mod oauth;
pub mod token;

use serde::Deserialize;

pub type OAuthResult<T, E = anyhow::Error> = anyhow::Result<T, E>;

#[derive(thiserror::Error, Deserialize, Debug)]
pub enum OAuthError {
    #[error("Invalid request (error {error:?}, error_description {error_description:?})")]
    BadRequest {
        error: String,
        error_description: String,
    },
    #[error("Failed to get public key")]
    FailedPubKeyRequest,
    #[error("Failed login")]
    FailedLogin,
    #[error("Failed logged in")]
    FailedLoginIn,
    #[error("Failed get code from callback url")]
    FailedCallbackCode,
    #[error("Failed callback url")]
    FailedCallbackURL,
    #[error("Invalid request login url")]
    InvalidLoginUrl,
    #[error("Invalid email or password")]
    InvalidEmailOrPassword,
    #[error("Invalid email")]
    InvalidEmail,
    #[error("Invalid Location")]
    InvalidLocation,
    #[error("Invalid token")]
    InvalidAccessToken,
    #[error("Token expired")]
    TokenExipired,
    #[error("Invalid MFA code")]
    InvalidMFACode,
    #[error("MFA failed")]
    MFAFailed,
    #[error("MFA required")]
    MFARequired,
}
