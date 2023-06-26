use std::{ops::Not, path::PathBuf};

use crate::{model::AuthenticateToken, OAuthError, TokenStoreError};
use anyhow::Context;

use jsonwebtokens::{Algorithm, AlgorithmID, Verifier};

use std::{collections::HashMap, sync::RwLock};

use async_trait::async_trait;

pub const PUBLIC_KEY: &[u8] = "-----BEGIN PUBLIC KEY-----\n\
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
-----END PUBLIC KEY-----"
    .as_bytes();

pub type TokenResult<T, E = anyhow::Error> = anyhow::Result<T, E>;

#[async_trait]
pub trait AuthenticateTokenStore: Send + Sync {
    /// Store Authenticate Token return an old Token
    async fn set_token(
        &mut self,
        token: AuthenticateToken,
    ) -> TokenResult<Option<AuthenticateToken>>;

    /// Read Authenticate Token return a copy of the Token
    async fn get_token(&self, email: &str) -> TokenResult<Option<AuthenticateToken>>;

    /// Delete Authenticate Token return an current Token
    async fn delete_token(&mut self, email: &str) -> TokenResult<Option<AuthenticateToken>>;

    /// List Authenticate Token
    async fn token_list(&self) -> TokenResult<Vec<AuthenticateToken>>;
}

#[derive(Debug)]
pub struct TokenMemStore(RwLock<HashMap<String, AuthenticateToken>>);

impl Default for TokenMemStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenMemStore {
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
        TokenMemStore(RwLock::new(HashMap::new()))
    }
}

#[async_trait]
impl AuthenticateTokenStore for TokenMemStore {
    async fn set_token(
        &mut self,
        token: AuthenticateToken,
    ) -> TokenResult<Option<AuthenticateToken>> {
        Ok(self
            .0
            .write()
            .map_err(|_| TokenStoreError::AccessError)?
            .insert(token.email().to_string(), token))
    }

    async fn get_token(&self, email: &str) -> TokenResult<Option<AuthenticateToken>> {
        let binding = self.0.read().map_err(|_| TokenStoreError::AccessError)?;
        Ok(binding.get(email).cloned())
    }

    async fn delete_token(&mut self, email: &str) -> TokenResult<Option<AuthenticateToken>> {
        Ok(self
            .0
            .write()
            .map_err(|_| TokenStoreError::AccessError)?
            .remove(email))
    }

    async fn token_list(&self) -> TokenResult<Vec<AuthenticateToken>> {
        let binding = self.0.read().map_err(|_| TokenStoreError::AccessError)?;
        let list = binding
            .values()
            .map(|v| v.clone())
            .collect::<Vec<AuthenticateToken>>();
        Ok(list)
    }
}

pub struct TokenFileStore(PathBuf);

impl Default for TokenFileStore {
    fn default() -> Self {
        let default_path = PathBuf::from(crate::DEFAULT_TOKEN_FILE);
        if default_path.exists().not() {
            std::fs::File::create(&default_path).unwrap_or_else(|_| {
                panic!(
                    "{}",
                    TokenStoreError::CreateDefaultTokenFileError.to_string()
                )
            });
        }
        TokenFileStore(default_path)
    }
}

impl TokenFileStore {
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
        Ok(TokenFileStore(path))
    }
}

#[async_trait]
impl AuthenticateTokenStore for TokenFileStore {
    async fn set_token(
        &mut self,
        token: AuthenticateToken,
    ) -> TokenResult<Option<AuthenticateToken>> {
        verify_access_token(&token.access_token())
            .await
            .context(TokenStoreError::AccessTokenVerifyError)?;
        let bytes = tokio::fs::read(&self.0).await?;
        let mut data: HashMap<String, AuthenticateToken> = if bytes.is_empty() {
            HashMap::new()
        } else {
            serde_json::from_slice(&bytes).map_err(|_| TokenStoreError::AccessError)?
        };
        let v = data.insert(token.email().to_string(), token);
        let json = serde_json::to_string_pretty(&data)?;
        tokio::fs::write(&self.0, json.as_bytes()).await?;
        Ok(v)
    }

    async fn get_token(&self, email: &str) -> TokenResult<Option<AuthenticateToken>> {
        let bytes = tokio::fs::read(&self.0).await?;
        if bytes.is_empty() {
            return Ok(None);
        }
        let data: HashMap<String, AuthenticateToken> =
            serde_json::from_slice(&bytes).map_err(TokenStoreError::DeserializeError)?;
        Ok(data.get(email).cloned())
    }

    async fn delete_token(&mut self, email: &str) -> TokenResult<Option<AuthenticateToken>> {
        let bytes = tokio::fs::read(&self.0).await?;
        if bytes.is_empty() {
            return Ok(None);
        }
        let mut data: HashMap<String, AuthenticateToken> =
            serde_json::from_slice(&bytes).map_err(|_| TokenStoreError::AccessError)?;
        let v = data.remove(email);
        let json = serde_json::to_string_pretty(&data)?;
        tokio::fs::write(&self.0, json).await?;
        Ok(v)
    }

    async fn token_list(&self) -> TokenResult<Vec<AuthenticateToken>> {
        let bytes = tokio::fs::read(&self.0).await?;
        if bytes.is_empty() {
            return Ok(vec![]);
        }
        let data: HashMap<String, AuthenticateToken> =
            serde_json::from_slice(&bytes).map_err(TokenStoreError::DeserializeError)?;
        let list = data
            .values()
            .map(|v| v.clone())
            .collect::<Vec<AuthenticateToken>>();
        Ok(list)
    }
}

#[cfg(feature = "remote-token")]
#[derive(Deserialize)]
struct Keys {
    alg: String,
    x5c: Vec<String>,
}

#[cfg(feature = "remote-token")]
#[derive(Deserialize)]
struct KeyResult {
    keys: Vec<Keys>,
}

#[cfg(feature = "remote--token")]
async fn keys() -> TokenResult<KeyResult> {
    use reqwest::header;
    use std::str::FromStr;
    let client = reqwest::Client::builder()
        .user_agent("ChatGPT/1.2023.21 (iOS 16.2; iPad11,1; build 623)")
        .timeout(std::time::Duration::from_secs(3))
        .build()?;
    let resp = client
        .get("https://auth0.openai.com/.well-known/jwks.json")
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
    let claims: serde_json::Value = verifier.verify(token, &alg)?;
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

pub async fn verify_access_token_for_u8(token: &[u8]) -> TokenResult<()> {
    let x = String::from_utf8(token.to_vec())?;
    verify_access_token(&x).await
}

pub async fn verify_access_token(token: &str) -> TokenResult<()> {
    let token = token.trim_start_matches("Bearer ");
    if token.starts_with("sk-") || token.starts_with("sess-") {
        return Ok(());
    }
    match verify(token, PUBLIC_KEY, AlgorithmID::RS256) {
        Ok(_) => Ok(()),
        #[cfg(not(feature = "remote-token"))]
        Err(err) => Err(err),
        #[cfg(feature = "remote-token")]
        Err(_) => {
            let key_result = keys().await?;
            let key = key_result
                .keys
                .first()
                .ok_or(OAuthError::FailedPubKeyRequest)?;
            let pub_key = key.x5c.first().ok_or(OAuthError::FailedPubKeyRequest)?;
            let pub_key = format!("-----BEGIN PUBLIC KEY-----{pub_key}-----END PUBLIC KEY-----");
            let alg = AlgorithmID::from_str(key.alg.as_str())?;
            verify(token, pub_key.as_bytes(), alg)
        }
    }
}
