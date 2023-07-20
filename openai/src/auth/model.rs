use super::AuthStrategy;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize, Builder)]
pub struct AuthAccount {
    pub username: String,
    pub password: String,
    #[builder(setter(into, strip_option), default)]
    pub mfa: Option<String>,
    pub option: AuthStrategy,
    #[builder(setter(into, strip_option), default)]
    #[serde(rename = "cf-turnstile-response")]
    pub cf_turnstile_response: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct AppleAccessToken {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub expires_in: i64,
}

#[derive(Serialize, Deserialize)]
pub struct RefreshToken {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    pub id_token: String,
    pub expires_in: i64,
}

#[derive(Deserialize)]
pub struct DashSession {
    pub object: String,
    pub user: DashUser,
    pub invites: Vec<Value>,
}

impl DashSession {
    pub fn sensitive_id(&self) -> &str {
        &self.user.session.sensitive_id
    }

    pub fn user_id(&self) -> &str {
        &self.user.id
    }

    pub fn nickname(&self) -> &str {
        &self.user.name
    }

    pub fn email(&self) -> &str {
        &self.user.email
    }

    pub fn picture(&self) -> &str {
        &self.user.picture
    }
}

#[derive(Deserialize)]
pub struct DashUser {
    pub object: String,
    pub id: String,
    pub email: String,
    pub name: String,
    pub picture: String,
    pub created: i64,
    pub groups: Vec<Value>,
    pub session: DashUserSession,
    pub orgs: DashUserOrgs,
    pub intercom_hash: String,
    pub amr: Vec<Value>,
}

#[derive(Deserialize, Serialize)]
pub struct DashUserSession {
    pub sensitive_id: String,
    pub object: String,
    pub name: Option<String>,
    pub created: i64,
    pub last_use: Option<i64>,
    pub publishable: bool,
}

#[derive(Deserialize)]
pub struct DashUserOrgs {
    pub object: String,
    pub data: Vec<DashUserOrgsData>,
}

#[derive(Deserialize)]
pub struct DashUserOrgsData {
    pub object: String,
    pub id: String,
    pub created: i64,
    pub title: String,
    pub name: String,
    pub description: String,
    pub personal: bool,
    pub is_default: bool,
    pub role: String,
    pub groups: Vec<Value>,
}

#[derive(Deserialize)]
pub struct ApiKey {
    pub result: String,
    pub key: Option<Key>,
}

#[derive(Deserialize)]
pub struct ApiKeyList {
    pub object: String,
    pub data: Vec<Key>,
}

#[derive(Deserialize)]
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

#[derive(Serialize, Deserialize)]
pub struct WebAccessToken {
    pub user: WebUser,
    pub expires: String,
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "authProvider")]
    pub auth_provider: String,
}

#[derive(Deserialize, Serialize)]
pub struct WebUser {
    pub id: String,
    pub name: String,
    pub email: String,
    pub image: String,
    pub picture: String,
    pub idp: String,
    pub iat: i64,
    pub mfa: bool,
    pub groups: Vec<Value>,
    #[serde(rename = "intercom_hash")]
    pub intercom_hash: String,
}

#[derive(Serialize)]
pub enum AccessTokenOption {
    Web(WebAccessToken),
    Apple(AppleAccessToken),
}
