use std::str::FromStr;

use crate::OAuthError;
use jsonwebtokens::{Algorithm, AlgorithmID, Verifier};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const OAUTH_PUBLIC_KEY_URL: &str = "https://auth0.openai.com/.well-known/jwks.json";
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

#[derive(Serialize, Deserialize)]
struct Keys {
    alg: String,
    x5c: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct KeyResult {
    keys: Vec<Keys>,
}

async fn keys() -> anyhow::Result<KeyResult> {
    use reqwest::Client;
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()?;
    let resp = client.get(OAUTH_PUBLIC_KEY_URL).send().await?;
    if resp.status().is_success() {
        let keys = resp.json::<KeyResult>().await?;
        return Ok(keys);
    }
    anyhow::bail!(OAuthError::FailedPubKeyRequest)
}

fn verify(token: &str, pub_key: &[u8], alg: AlgorithmID) -> anyhow::Result<()> {
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
    anyhow::bail!(OAuthError::InvalidToken)
}

pub async fn verify_access_token(token: &str) -> anyhow::Result<()> {
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
