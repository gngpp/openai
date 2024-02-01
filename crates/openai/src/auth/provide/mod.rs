#[cfg(feature = "preauth")]
pub mod apple;
pub mod platform;
pub mod web;

use std::collections::HashSet;

use crate::{
    arkose::{self, ArkoseContext, ArkoseToken, Type},
    with_context,
};

use super::{
    error::AuthError,
    model::{self, AuthStrategy},
};
use reqwest::header;
use serde::Serialize;
use typed_builder::TypedBuilder;

pub type AuthResult<T, E = AuthError> = anyhow::Result<T, E>;

#[async_trait::async_trait]
pub trait AuthProvider: Send + Sync {
    async fn do_access_token(&self, account: &model::AuthAccount)
        -> AuthResult<model::AccessToken>;

    async fn do_revoke_token(&self, refresh_token: &str) -> AuthResult<()>;

    async fn do_refresh_token(&self, refresh_token: &str) -> AuthResult<model::RefreshToken>;

    fn supports(&self, t: &AuthStrategy) -> bool;
}

trait RequestContextExt {
    type Target;
    fn ext_context(self, ctx: &mut RequestContext) -> Self::Target;
}

impl RequestContextExt for reqwest::Response {
    type Target = reqwest::Response;
    fn ext_context(self, ctx: &mut RequestContext) -> Self::Target {
        ctx.add_cookie(self.cookies());
        self
    }
}

impl RequestContextExt for reqwest::RequestBuilder {
    type Target = reqwest::RequestBuilder;
    fn ext_context(self, ctx: &mut RequestContext) -> Self::Target {
        self.header(header::COOKIE, ctx.get_cookie())
    }
}

struct RequestContext<'a> {
    account: &'a model::AuthAccount,
    cookie: HashSet<String>,
    csrf_token: String,
    state: String,
    code_verifier: String,
    code_challenge: String,
}

impl<'a> RequestContext<'a> {
    pub(super) fn new(account: &'a model::AuthAccount) -> RequestContext<'a> {
        Self {
            account,
            cookie: HashSet::new(),
            csrf_token: String::new(),
            state: String::new(),
            code_verifier: String::new(),
            code_challenge: String::new(),
        }
    }

    fn add_cookie<'b>(&mut self, c: impl Iterator<Item = reqwest::cookie::Cookie<'b>> + 'b) {
        c.for_each(|v| {
            let _ = self.cookie.insert(format!("{}={}", v.name(), v.value()));
        });
    }

    fn get_cookie(&self) -> String {
        self.cookie.iter().cloned().collect::<Vec<_>>().join("; ")
    }

    fn set_csrf_token(&mut self, csrf_token: &str) {
        self.csrf_token = csrf_token.to_owned();
    }

    fn set_state(&mut self, state: &str) {
        if self.state.is_empty() {
            self.state.push_str(state);
        }
    }

    fn set_code_verifier(&mut self, code_verifier: String) {
        if self.code_verifier.is_empty() {
            self.code_verifier.push_str(&code_verifier);
        }
    }

    fn set_code_challenge(&mut self, code_challenge: String) {
        if self.code_challenge.is_empty() {
            self.code_challenge.push_str(&code_challenge);
        }
    }

    async fn load_arkose_token(&mut self) -> AuthResult<()> {
        let arkose_token = match self.account.arkose_token.as_deref() {
            Some(arkose_token) => {
                let arkose_token = ArkoseToken::from(arkose_token);
                let _ = arkose_token.callback().await;
                arkose_token
            }
            None => arkose::ArkoseToken::new_from_context(
                ArkoseContext::builder()
                    .client(with_context!(arkose_client))
                    .typed(Type::Auth)
                    .build(),
            )
            .await
            .map_err(AuthError::InvalidArkoseToken)?,
        };

        self.cookie
            .insert(format!("arkoseToken={}", arkose_token.value()));

        Ok(())
    }
}

#[derive(Serialize, TypedBuilder)]
struct IdentifierData<'a> {
    state: &'a str,
    username: &'a str,
    #[serde(rename = "js-available")]
    js_available: bool,
    #[serde(rename = "webauthn-available")]
    webauthn_available: bool,
    #[serde(rename = "is-brave")]
    is_brave: bool,
    #[serde(rename = "webauthn-platform-available")]
    webauthn_platform_available: bool,
    action: &'a str,
}

#[derive(Clone)]
enum GrantType {
    AuthorizationCode,
    RefreshToken,
}

impl Serialize for GrantType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            GrantType::AuthorizationCode => serializer.serialize_str("authorization_code"),
            GrantType::RefreshToken => serializer.serialize_str("refresh_token"),
        }
    }
}

#[derive(Serialize, TypedBuilder)]
struct AuthenticateData<'a> {
    state: &'a str,
    username: &'a str,
    password: &'a str,
    action: &'a str,
}

#[derive(Serialize, TypedBuilder)]
struct AuthenticateMfaData<'a> {
    state: &'a str,
    code: &'a str,
    action: &'a str,
}

#[derive(Serialize, TypedBuilder)]
struct AuthorizationCodeData<'a> {
    redirect_uri: &'a str,
    grant_type: GrantType,
    client_id: &'a str,
    code_verifier: Option<&'a str>,
    code: &'a str,
}

#[derive(Serialize, TypedBuilder)]
struct RevokeTokenData<'a> {
    client_id: &'a str,
    token: &'a str,
}

#[derive(Serialize, TypedBuilder)]
struct RefreshTokenData<'a> {
    redirect_uri: &'a str,
    grant_type: GrantType,
    client_id: &'a str,
    refresh_token: &'a str,
}

#[derive(Serialize, TypedBuilder)]
struct GetAuthorizedUrlData<'a> {
    #[serde(rename = "callbackUrl")]
    callback_url: &'a str,
    #[serde(rename = "csrfToken")]
    csrf_token: &'a str,
    json: &'a str,
}
