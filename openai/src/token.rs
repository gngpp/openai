use std::str::FromStr;

use crate::{OAuthError, TokenResult};
use jsonwebtokens::{Algorithm, AlgorithmID, Verifier};
use reqwest::header;
use serde::Deserialize;
use serde_json::Value;

pub const PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----\n\
MIIC+zCCAeOgAwIBAgIJLlfMWYK8snRdMA0GCSqGSIb3DQEBCwUAMBsxGTAXBgNVBAM\n\
TEG9wZW5haS5hdXRoMC5jb20wHhcNMjAwMjExMDUyMjI5WhcNMzMxMDIwMDUyMjI5Wj\n\
AbMRkwFwYDVQQDExBvcGVuYWkuYXV0aDAuY29tMIIBIjANBgkqhkiG9w0BAQEFAAOCA\n\
Q8AMIIBCgKCAQEA27rOErDOPvPc3mOADYtQBeenQm5NS5VHVaoO/Zmgsf1M0Wa/2WgL\n\
m9jX65Ru/K8Az2f4MOdpBxxLL686ZS+K7eJC/oOnrxCRzFYBqQbYo+JMeqNkrCn34ye\n\
d4XkX4ttoHi7MwCEpVfb05Qf/ZAmNI1XjecFYTyZQFrd9LjkX6lr05zY6aM/+MCBNeB\n\
Wp35pLLKhiq9AieB1wbDPcGnqxlXuU/bLgIyqUltqLkr9JHsf/2T4VrXXNyNeQyBq5w\n\
jYlRkpBQDDDNOcdGpx1buRrZ2hFyYuXDRrMcR6BQGC0ur9hI5obRYlchDFhlb0ElsJ2\n\
bshDDGRk5k3doHqbhj2IgQIDAQABo0IwQDAPBgNVHRMBAf8EBTADAQH/MB0GA1UdDgQ\n\
WBBSzpMyU3UZWR9zdv+ckg/L6GZCcJDAOBgNVHQ8BAf8EBAMCAoQwDQYJKoZIhvcNAQ\n\
ELBQADggEBAEuUscoo1BZmCUZG8TEki0NHFjv08u2SHdcMU1xR0PfyKY6h+pLrSrGq8\n\
kYfjCHb/OPt0+Han0fiGRTnKurQ/u1leuJ7qHVHRILmP3e1MC8PUELjHpBo3f38Kk6U\n\
lbR5pbL5K7ZHeEO6CLNTOg54xLY/6e2ben4wv/LP39E6Gg56+iT/goJHkV64+nu3v3d\n\
Tmj+uSHWfkq93oG5tsOk2nTN4UCpyT5fWGv4eh7q2cKElMQM5GT/uZnCjEdDmJU2M11\n\
k6Ttg+FMNPgvH6R4e+lqhtmslXwXv9Xm95eS6JokJaYUimNX+dzhD+eRq+88vGJO63s\n\
afkEyGvifAMJFPwO78=\n\
-----END PUBLIC KEY-----";
const OAUTH_PUBLIC_KEY_URL: &str = "https://auth0.openai.com/.well-known/jwks.json";
const UA: &str = "ChatGPT/1.2023.21 (iOS 16.2; iPad11,1; build 623)";

#[derive(Deserialize)]
struct Keys {
    alg: String,
    x5c: Vec<String>,
}

#[derive(Deserialize)]
struct KeyResult {
    keys: Vec<Keys>,
}

async fn keys() -> TokenResult<KeyResult> {
    use reqwest::Client;
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()?;
    let resp = client.get(OAUTH_PUBLIC_KEY_URL)
    .header(header::USER_AGENT, header::HeaderValue::from_static(UA))
    .send().await?;
    if resp.status().is_success() {
        let keys = resp.json::<KeyResult>().await?;
        return Ok(keys);
    }
    anyhow::bail!(OAuthError::FailedPubKeyRequest)
}

fn verify(token: &str, pub_key: &[u8], alg: AlgorithmID) -> TokenResult<()> {
    let alg = Algorithm::new_rsa_pem_verifier(alg, pub_key)?;
    let verifier = Verifier::create().build()?;
    let claims: Value = verifier.verify(&token, &alg)?;
    let claims_str = claims.to_string();
    if claims_str.contains("https://openai.openai.auth0app.com/userinfo")
        && claims_str.contains("https://auth0.openai.com/")
        && claims_str.contains("https://api.openai.com/v1")
        && claims_str.contains("model.read")
        && claims_str.contains("model.request")
    {
        return Ok(());
    }
    anyhow::bail!(OAuthError::InvalidAccessToken)
}

pub async fn verify_access_token(token: &str) -> TokenResult<()> {
    if token.starts_with("sk-") {
        return Ok(());
    }
    match verify(token, PUBLIC_KEY.as_bytes(), AlgorithmID::RS256) {
        Ok(_) => Ok(()),
        Err(_) => {
            let key_result = keys().await?;
            let key = key_result
                .keys
                .first()
                .ok_or(OAuthError::FailedPubKeyRequest)?;
            let pub_key = key.x5c.first().ok_or(OAuthError::FailedPubKeyRequest)?;
            let pub_key = format!(
                "-----BEGIN PUBLIC KEY-----{}-----END PUBLIC KEY-----",
                pub_key
            );
            let alg = AlgorithmID::from_str(key.alg.as_str())?;
            verify(token, pub_key.as_bytes(), alg)
        }
    }
}

use std::{collections::HashMap, sync::RwLock};

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait AccessTokenStore: Send + Sync {
    // Store AccessToken return an old Token
    async fn set_access_token(&mut self, token: Token) -> TokenResult<Option<Token>>;

    // Read AccessToken return a copy of the Token
    async fn get_access_token(&self, email: &String) -> TokenResult<Option<Token>>;

    // Delete AccessToken return an current Token
    async fn delete_access_token(&mut self, email: &String) -> TokenResult<Option<Token>>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Token {
    email: String,
    access_token: String,
    refresh_token: String,
    id_token: String,
    expired_in: u32,
}

static mut MEM_STORAGE: std::mem::MaybeUninit<MemStore> = std::mem::MaybeUninit::uninit();
static mut FILE_STORAGE: std::mem::MaybeUninit<FileStore> = std::mem::MaybeUninit::uninit();
static ONCE: std::sync::Once = std::sync::Once::new();

#[derive(thiserror::Error, Debug)]
pub enum TokenStoreError {
    #[error("[TokenStoreError] AccessError")]
    AccessError,
    #[error("[TokenStoreError] NotFoundError")]
    NotFoundError,
}

#[derive(Debug)]
pub struct MemStore(RwLock<HashMap<String, Token>>);

#[async_trait]
impl AccessTokenStore for MemStore {
    async fn set_access_token(&mut self, token: Token) -> TokenResult<Option<Token>> {
        Ok(self
            .0
            .write()
            .map_err(|_| TokenStoreError::AccessError)?
            .insert(token.email.to_string(), token))
    }

    async fn get_access_token(&self, email: &String) -> TokenResult<Option<Token>> {
        let binding = self.0.read().map_err(|_| TokenStoreError::AccessError)?;
        Ok(binding.get(email).cloned())
    }

    async fn delete_access_token(&mut self, email: &String) -> TokenResult<Option<Token>> {
        Ok(self
            .0
            .write()
            .map_err(|_| TokenStoreError::AccessError)?
            .remove(email))
    }
}

pub struct FileStore(std::sync::RwLock<HashMap<String, Token>>);

#[async_trait]
impl AccessTokenStore for FileStore {
    async fn set_access_token(&mut self, token: Token) -> TokenResult<Option<Token>> {
        todo!()
    }

    async fn get_access_token(&self, email: &String) -> TokenResult<Option<Token>> {
        todo!()
    }

    async fn delete_access_token(&mut self, email: &String) -> TokenResult<Option<Token>> {
        todo!()
    }
}

/// The `Policy` struct.
/// Implementation of a universal singleton storage policy
/// # Example
///
/// ```rust
/// let _ = Policy::mem_store();
/// ```
pub struct Policy;

impl Policy {
    
    /// # Examples
    ///
    /// ```
    /// let mut auth = openai::oauth::OpenOAuth0Builder::builder()
    ///    .email("opengpt@gmail.com".to_string())
    ///    .password("gngpp".to_string())
    ///    .cache(true)
    ///    .cookie_store(true)
    ///    .token_store(openai::token::Policy::men_store())
    ///    .client_timeout(std::time::Duration::from_secs(20))
    ///    .build();
    /// let token = auth.authenticate().await?;
    /// println!("Token: {}", token);
    /// println!("Profile: {:#?}", auth.get_user_info()?);
    /// ```
    pub fn men_store() -> impl AccessTokenStore {
        ONCE.call_once(|| unsafe {
            MEM_STORAGE
                .as_mut_ptr()
                .write(MemStore(RwLock::new(HashMap::new())))
        });
        unsafe { MEM_STORAGE.as_mut_ptr().read() }
    }

    /// # Examples
    ///
    /// ```
    /// let mut auth = openai::oauth::OpenOAuth0Builder::builder()
    ///    .email("opengpt@gmail.com".to_string())
    ///    .password("gngpp".to_string())
    ///    .cache(true)
    ///    .cookie_store(true)
    ///    .token_store(openai::token::Policy::file_store())
    ///    .client_timeout(std::time::Duration::from_secs(20))
    ///    .build();
    /// let token = auth.authenticate().await?;
    /// println!("Token: {}", token);
    /// println!("Profile: {:#?}", auth.get_user_info()?);
    /// ```
    pub fn file_store() -> impl AccessTokenStore {
        ONCE.call_once(|| unsafe {
            FILE_STORAGE
                .as_mut_ptr()
                .write(FileStore(RwLock::new(HashMap::new())))
        });
        unsafe { FILE_STORAGE.as_mut_ptr().read() }
    }
}
