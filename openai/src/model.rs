use serde::{Deserialize, Serialize};
use serde_json::Value;

use base64::{engine::general_purpose, Engine};

use crate::OAuthError;

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

    pub fn bearer_access_token(&self) -> String {
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
        let expires = (chrono::Utc::now() + chrono::Duration::seconds(value.expires_in)
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
        let expires = (chrono::Utc::now() + chrono::Duration::seconds(value.expires_in)
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticateSession {
    pub object: String,
    pub user: User,
    pub invites: Vec<Value>,
}

impl AuthenticateSession {
    pub fn sensitive_id(&self) -> &str {
        &self.user.session.sensitive_id
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub object: String,
    pub id: String,
    pub email: String,
    pub name: String,
    pub picture: String,
    pub created: i64,
    pub groups: Vec<Value>,
    pub session: Session,
    pub orgs: Orgs,
    #[serde(rename = "intercom_hash")]
    pub intercom_hash: String,
    pub amr: Vec<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    #[serde(rename = "sensitive_id")]
    pub sensitive_id: String,
    pub object: String,
    pub name: Value,
    pub created: i64,
    #[serde(rename = "last_use")]
    pub last_use: i64,
    pub publishable: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Orgs {
    pub object: String,
    pub data: Vec<Daum>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Daum {
    pub object: String,
    pub id: String,
    pub created: i64,
    pub title: String,
    pub name: String,
    pub description: String,
    pub personal: bool,
    #[serde(rename = "is_default")]
    pub is_default: bool,
    pub role: String,
    pub groups: Vec<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticateApiKey {
    pub result: String,
    pub key: Option<Key>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticateApiKeyList {
    pub object: String,
    pub data: Vec<Key>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Key {
    #[serde(rename = "sensitive_id")]
    pub sensitive_id: String,
    pub object: String,
    pub name: String,
    pub created: i64,
    #[serde(rename = "last_use")]
    pub last_use: Value,
    pub publishable: bool,
}
