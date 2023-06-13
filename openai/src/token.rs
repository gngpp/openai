use std::{ops::Not, path::PathBuf, str::FromStr};

use crate::{OAuthError, TokenResult, TokenStoreError};
use anyhow::Context;
use base64::{engine::general_purpose, Engine};
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

use std::{collections::HashMap, sync::RwLock};

use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait AuthenticateTokenStore: Send + Sync {
    // Store Authenticate Token return an old Token
    async fn set_token(
        &mut self,
        token: AuthenticateToken,
    ) -> TokenResult<Option<AuthenticateToken>>;

    // Read Authenticate Token return a copy of the Token
    async fn get_token(&self, email: &String) -> TokenResult<Option<AuthenticateToken>>;

    // Delete Authenticate Token return an current Token
    async fn delete_token(&mut self, email: &String) -> TokenResult<Option<AuthenticateToken>>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthenticateToken {
    access_token: String,
    refresh_token: String,
    expires: i64,
    profile: Profile,
}

impl AuthenticateToken {
    pub fn email(&self) -> &str {
        &self.profile.email
    }

    pub fn access_token(&self) -> &str {
        &self.access_token
    }

    pub fn get_bearer_access_token(&self) -> String {
        format!("Bearer {}", &self.access_token)
    }
    pub fn refresh_token(&self) -> &str {
        &self.refresh_token
    }

    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.expires
    }

    pub fn profile(&self) -> &Profile {
        &self.profile
    }
}

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Profile {
    pub nickname: String,
    pub name: String,
    pub picture: String,
    pub updated_at: String,
    pub email_verified: bool,
    pub email: String,
    pub iss: String,
    pub aud: String,
    pub iat: i64,
    pub exp: i64,
    pub sub: String,
    pub auth_time: i64,
}

impl TryFrom<String> for Profile {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let split_jwt_strings: Vec<_> = value.split('.').collect();
        let jwt_body = split_jwt_strings
            .get(1)
            .ok_or(OAuthError::InvalidAccessToken)?;
        let decoded_jwt_body = general_purpose::URL_SAFE_NO_PAD.decode(jwt_body)?;
        let converted_jwt_body = String::from_utf8(decoded_jwt_body)?;
        let profile = serde_json::from_str::<Profile>(&converted_jwt_body)?;
        Ok(profile)
    }
}

impl TryFrom<crate::oauth::AccessToken> for AuthenticateToken {
    type Error = anyhow::Error;

    fn try_from(value: crate::oauth::AccessToken) -> Result<Self, Self::Error> {
        let profile = Profile::try_from(value.id_token)?;
        let expires = (chrono::Utc::now() + chrono::Duration::seconds(i64::from(value.expires_in))
            - chrono::Duration::minutes(5))
        .timestamp();
        Ok(Self {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            expires,
            profile,
        })
    }
}

impl TryFrom<crate::oauth::RefreshToken> for AuthenticateToken {
    type Error = anyhow::Error;

    fn try_from(value: crate::oauth::RefreshToken) -> Result<Self, Self::Error> {
        let profile = Profile::try_from(value.id_token)?;
        let expires = (chrono::Utc::now() + chrono::Duration::seconds(i64::from(value.expires_in))
            - chrono::Duration::minutes(5))
        .timestamp();
        Ok(Self {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            expires,
            profile,
        })
    }
}

#[derive(Debug)]
pub struct MemStore(RwLock<HashMap<String, AuthenticateToken>>);

impl MemStore {
    /// # Examples
    ///
    /// ```
    /// let mut auth = openai::oauth::OpenOAuth0Builder::builder()
    ///    .email("opengpt@gmail.com".to_string())
    ///    .password("gngpp".to_string())
    ///    .cache(true)
    ///    .cookie_store(true)
    ///    .token_store(openai::token::MemStore::new())
    ///    .client_timeout(std::time::Duration::from_secs(20))
    ///    .build();
    /// let token = auth.authenticate().await?;
    /// println!("Token: {}", token);
    /// println!("Profile: {:#?}", auth.get_user_info()?);
    /// ```
    pub fn new() -> Self {
        MemStore(RwLock::new(HashMap::new()))
    }
}

#[async_trait]
impl AuthenticateTokenStore for MemStore {
    async fn set_token(
        &mut self,
        token: AuthenticateToken,
    ) -> TokenResult<Option<AuthenticateToken>> {
        Ok(self
            .0
            .write()
            .map_err(|_| TokenStoreError::AccessError)?
            .insert(token.profile.email.to_string(), token))
    }

    async fn get_token(&self, email: &String) -> TokenResult<Option<AuthenticateToken>> {
        let binding = self.0.read().map_err(|_| TokenStoreError::AccessError)?;
        Ok(binding.get(email).cloned())
    }

    async fn delete_token(&mut self, email: &String) -> TokenResult<Option<AuthenticateToken>> {
        Ok(self
            .0
            .write()
            .map_err(|_| TokenStoreError::AccessError)?
            .remove(email))
    }
}

pub struct FileStore(PathBuf);

impl Default for FileStore {
    fn default() -> Self {
        let default_path = PathBuf::from(crate::DEFAULT_TOKEN_FILE);
        if default_path.exists().not() {
            std::fs::File::create(&default_path)
                .expect(&TokenStoreError::CreateDefaultTokenFileError.to_string());
        }
        FileStore(default_path)
    }
}

impl FileStore {
    pub async fn new(path: Option<PathBuf>) -> TokenResult<Self> {
        let path = path.unwrap_or(Default::default());
        if let Some(parent) = path.parent() {
            if path.exists().not() {
                tokio::fs::create_dir_all(parent).await?
            }
        }
        if path.exists().not() {
            tokio::fs::File::create(&path).await?;
        }
        Ok(FileStore(path))
    }
}

#[async_trait]
impl AuthenticateTokenStore for FileStore {
    async fn set_token(
        &mut self,
        token: AuthenticateToken,
    ) -> TokenResult<Option<AuthenticateToken>> {
        verify_access_token(&token.access_token)
            .await
            .context(TokenStoreError::AccessTokenVerifyError)?;
        let bytes = tokio::fs::read(&self.0).await?;
        let mut data: HashMap<String, AuthenticateToken> = if bytes.len() == 0 {
            HashMap::new()
        } else {
            serde_json::from_slice(&bytes).map_err(|_| TokenStoreError::AccessError)?
        };
        let v = data.insert(token.profile.email.to_string(), token);
        let json = serde_json::to_string_pretty(&data)?;
        tokio::fs::write(&self.0, json.as_bytes()).await?;
        Ok(v)
    }

    async fn get_token(&self, email: &String) -> TokenResult<Option<AuthenticateToken>> {
        let bytes = tokio::fs::read(&self.0).await?;
        if bytes.len() == 0 {
            return Ok(None);
        }
        let data: HashMap<String, AuthenticateToken> =
            serde_json::from_slice(&bytes).map_err(TokenStoreError::DeserializeError)?;
        Ok(data.get(email).cloned())
    }

    async fn delete_token(&mut self, email: &String) -> TokenResult<Option<AuthenticateToken>> {
        let bytes = tokio::fs::read(&self.0).await?;
        if bytes.len() == 0 {
            return Ok(None);
        }
        let mut data: HashMap<String, AuthenticateToken> =
            serde_json::from_slice(&bytes).map_err(|_| TokenStoreError::AccessError)?;
        let v = data.remove(email);
        let json = serde_json::to_string_pretty(&data)?;
        tokio::fs::write(&self.0, json).await?;
        Ok(v)
    }
}

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
    let resp = client
        .get(OAUTH_PUBLIC_KEY_URL)
        .header(header::USER_AGENT, header::HeaderValue::from_static(UA))
        .send()
        .await?;
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
