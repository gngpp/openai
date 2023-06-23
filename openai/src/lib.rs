pub mod api;
#[cfg(feature = "stream")]
pub mod eventsource;
pub mod log;
pub mod model;
pub mod oauth;
pub mod unescape;

#[cfg(feature = "serve")]
pub mod serve;
pub mod token;

pub const DEFAULT_TOKEN_FILE: &str = ".opengpt-access_tokens";
pub type OAuthResult<T, E = anyhow::Error> = anyhow::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum OAuthError {
    #[error("other request (error {0:?}")]
    Other(String),
    #[error("token access (error {0:?}")]
    TokenAccess(anyhow::Error),
    #[error("bad request (error {0:?}")]
    BadRequest(String),
    #[error("too many requests `{0}`")]
    TooManyRequests(String),
    #[error("Unauthorized request (error {0:?}")]
    Unauthorized(String),
    #[error("Unauthorized request (error {0:?}")]
    ServerError(String),
    #[error("failed to get public key")]
    FailedPubKeyRequest,
    #[error("failed login")]
    FailedLogin,
    #[error(transparent)]
    FailedRequest(#[from] reqwest::Error),
    #[error("failed logged in")]
    FailedLoginIn,
    #[error("failed get code from callback url")]
    FailedCallbackCode,
    #[error("failed callback url")]
    FailedCallbackURL,
    #[error("invalid request login url (error {0:?}")]
    InvalidLoginUrl(String),
    #[error("invalid email or password")]
    InvalidEmailOrPassword,
    #[error("invalid request {0:?}")]
    InvalidRequest(String),
    #[error("invalid email")]
    InvalidEmail,
    #[error("invalid Location")]
    InvalidLocation,
    #[error("invalid access-token")]
    InvalidAccessToken,
    #[error("invalid refresh-token")]
    InvalidRefreshToken,
    #[error("token expired")]
    TokenExpired,
    #[error("MFA failed")]
    MFAFailed,
    #[error("MFA required")]
    MFARequired,
    #[error("json deserialize error `{0}`")]
    DeserializeError(String),
}

#[derive(thiserror::Error, Debug)]
pub enum TokenStoreError {
    #[error("failed to access token")]
    AccessError,
    #[error("token not found error")]
    NotFoundError,
    #[error("failed token deserialize")]
    DeserializeError(#[from] serde_json::error::Error),
    #[error("failed to verify access_token")]
    AccessTokenVerifyError,
    #[error("failed to create default token store file")]
    CreateDefaultTokenFileError,
}
