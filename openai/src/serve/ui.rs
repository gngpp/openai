use std::collections::HashMap;
use std::sync::Once;

use actix_web::cookie;
use actix_web::{
    cookie::Cookie, error, http::header, web, HttpRequest, HttpResponse, Responder, Result,
};
use base64::Engine;
use chrono::NaiveDateTime;
use chrono::{prelude::DateTime, Utc};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    auth::{model::AuthAccount, AuthHandle},
    model::AuthenticateToken,
    URL_CHATGPT_API,
};

use super::auth_client;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

const DEFAULT_INDEX: &str = "/";
const SESSION_ID: &str = "opengpt_session";
const BUILD_ID: &str = "XmKrBoPpskgF_4RiIX1jm";
const TEMP_404: &str = "404.htm";
const TEMP_AUTH: &str = "auth.htm";
const TEMP_CHAT: &str = "chat.htm";
const TEMP_DETAIL: &str = "detail.htm";
const TEMP_LOGIN: &str = "login.htm";
const TEMP_SHARE: &str = "share.htm";

static INIT: Once = Once::new();

pub(super) static mut STATIC_FILES: Option<HashMap<&'static str, static_files::Resource>> = None;
pub(super) static mut TEMPLATE: Option<tera::Tera> = None;
pub(super) static mut TEMPLATE_DATA: Option<TemplateData> = None;

#[derive(Serialize, Deserialize)]
struct Session {
    refresh_token: Option<String>,
    access_token: String,
    user_id: String,
    email: String,
    picture: Option<String>,
    expires_in: i64,
    expires: i64,
}

impl ToString for Session {
    fn to_string(&self) -> String {
        let json = serde_json::to_string(self)
            .expect("An error occurred during the internal serialization session");
        base64::engine::general_purpose::URL_SAFE.encode(json.as_bytes())
    }
}

impl TryFrom<&str> for Session {
    type Error = error::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        let data = base64::engine::general_purpose::URL_SAFE
            .decode(value)
            .map_err(|err| error::ErrorUnauthorized(err.to_string()))?;
        serde_json::from_slice(&data).map_err(|err| error::ErrorUnauthorized(err.to_string()))
    }
}

impl From<AuthenticateToken> for Session {
    fn from(value: AuthenticateToken) -> Self {
        let refresh_token = if let Some(refresh_token) = value.refresh_token() {
            Some(refresh_token.to_owned())
        } else {
            None
        };
        Session {
            user_id: value.user_id().to_owned(),
            email: value.email().to_owned(),
            picture: Some(value.picture().to_owned()),
            access_token: value.access_token().to_owned(),
            expires_in: value.expires_in(),
            expires: value.expires(),
            refresh_token,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub(super) struct TemplateData {
    /// WebSite api prefix
    api_prefix: Option<String>,
    /// Cloudflare captcha site key
    cf_site_key: Option<String>,
    /// Cloudflare captcha secret key
    cf_secret_key: Option<String>,
}

impl From<super::Launcher> for TemplateData {
    fn from(value: super::Launcher) -> Self {
        Self {
            api_prefix: Some(value.api_prefix.unwrap_or("".to_owned())),
            cf_site_key: value.cf_site_key,
            cf_secret_key: value.cf_secret_key,
        }
    }
}

// this function could be located in a different module
pub fn config(cfg: &mut web::ServiceConfig) {
    if !unsafe { super::DISABLE_UI } {
        let mut tera = tera::Tera::default();
        tera.add_raw_templates(vec![
            (TEMP_404, include_str!("../../ui/404.htm")),
            (TEMP_AUTH, include_str!("../../ui/auth.htm")),
            (TEMP_LOGIN, include_str!("../../ui/login.htm")),
            (TEMP_CHAT, include_str!("../../ui/chat.htm")),
            (TEMP_DETAIL, include_str!("../../ui/detail.htm")),
            (TEMP_SHARE, include_str!("../../ui/share.htm")),
        ])
        .expect("The static template failed to load");

        INIT.call_once(|| unsafe {
            STATIC_FILES = Some(generate());
            TEMPLATE = Some(tera);
        });

        cfg.route("/auth", web::get().to(get_auth))
            .route("/auth/login", web::get().to(get_login))
            .route("/auth/login", web::post().to(post_login))
            .route("/auth/login/token", web::post().to(post_login_token))
            .route("/auth/logout", web::get().to(get_logout))
            .route("/auth/session", web::get().to(get_session))
            .route("/", web::get().to(get_chat))
            .route("/c", web::get().to(get_chat))
            .route("/c/{conversation_id}", web::get().to(get_chat))
            .service(web::redirect("/chat", "/"))
            .service(web::redirect("/chat/{conversation_id}", "/"))
            .route("/share/{share_id}", web::get().to(get_share_chat))
            .route(
                "/share/{share_id}/continue",
                web::get().to(get_share_chat_continue),
            )
            .route(
                &format!("/_next/data/{BUILD_ID}/index.json"),
                web::get().to(get_chat_info),
            )
            .route(
                &format!("/_next/data/{BUILD_ID}/c/{}.json", "{conversation_id}"),
                web::get().to(get_chat_info),
            )
            .route(
                &format!("/_next/data/{BUILD_ID}/share/{}.json", "{share_id}"),
                web::get().to(get_share_chat_info),
            )
            .route(
                &format!(
                    "/_next/data/{BUILD_ID}/share/{}/continue.json",
                    "{share_id}"
                ),
                web::get().to(get_share_chat_continue_info),
            )
            // user picture
            .route("/_next/image", web::get().to(get_image))
            // static resource endpoints
            .route(
                "/{filename:.+\\.(png|js|css|webp|json)}",
                web::get().to(get_static_resource),
            )
            .route("/_next/static/{tail:.*}", web::to(get_static_resource))
            .route("/fonts/{tail:.*}", web::to(get_static_resource))
            .route("/ulp/{tail:.*}", web::to(get_static_resource))
            .route("/sweetalert2/{tail:.*}", web::to(get_static_resource))
            // 404 endpoint
            .default_service(web::route().to(get_error_404));
    }
}

async fn get_auth() -> Result<HttpResponse> {
    let mut ctx = tera::Context::new();
    settings_template_data(&mut ctx);
    render_template(TEMP_AUTH, &ctx)
}

async fn get_login() -> Result<HttpResponse> {
    let mut ctx = tera::Context::new();
    ctx.insert("error", "");
    ctx.insert("username", "");
    settings_template_data(&mut ctx);
    render_template(TEMP_LOGIN, &ctx)
}

async fn post_login(req: HttpRequest, account: web::Form<AuthAccount>) -> Result<HttpResponse> {
    let account = account.into_inner();
    if let Some(err) = cf_captcha_check(&req, account.cf_turnstile_response.as_deref())
        .await
        .err()
    {
        let mut ctx = tera::Context::new();
        ctx.insert("username", &account.username);
        ctx.insert("error", &err.to_string());
        return render_template(TEMP_LOGIN, &ctx);
    }
    match super::auth_client().do_access_token(&account).await {
        Ok(access_token) => {
            let authentication_token = AuthenticateToken::try_from(access_token)
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?;
            let session = Session::from(authentication_token);
            Ok(HttpResponse::SeeOther()
                .cookie(
                    Cookie::build(SESSION_ID, session.to_string())
                        .path(DEFAULT_INDEX)
                        .max_age(cookie::time::Duration::seconds(session.expires_in))
                        .same_site(cookie::SameSite::Lax)
                        .secure(false)
                        .http_only(false)
                        .finish(),
                )
                .append_header((header::LOCATION, DEFAULT_INDEX))
                .finish())
        }
        Err(e) => {
            let mut ctx = tera::Context::new();
            ctx.insert("username", &account.username);
            ctx.insert("error", &e.to_string());
            render_template(TEMP_LOGIN, &ctx)
        }
    }
}

async fn post_login_token(req: HttpRequest) -> Result<HttpResponse> {
    if let Some(token) = req.headers().get(header::AUTHORIZATION) {
        let access_token = token.to_str().unwrap_or_default();
        let profile = crate::token::check(access_token)
            .map_err(|e| error::ErrorUnauthorized(e.to_string()))?
            .ok_or(error::ErrorInternalServerError("Get Profile Erorr"))?;

        let session = match auth_client().do_get_user_picture(access_token).await {
            Ok(picture) => Session {
                refresh_token: None,
                access_token: access_token.to_owned(),
                user_id: profile.user_id().to_owned(),
                email: profile.email().to_owned(),
                picture: picture,
                expires_in: profile.expires_in(),
                expires: profile.expires(),
            },
            Err(_) => Session {
                user_id: profile.user_id().to_owned(),
                email: profile.email().to_owned(),
                picture: None,
                access_token: access_token.to_owned(),
                expires_in: profile.expires_in(),
                expires: profile.expires(),
                refresh_token: None,
            },
        };

        return Ok(HttpResponse::Ok()
            .insert_header((header::LOCATION, DEFAULT_INDEX))
            .cookie(
                Cookie::build(SESSION_ID, session.to_string())
                    .path(DEFAULT_INDEX)
                    .max_age(cookie::time::Duration::seconds(session.expires_in))
                    .same_site(cookie::SameSite::Lax)
                    .secure(false)
                    .http_only(false)
                    .finish(),
            )
            .finish());
    }
    redirect_login()
}

async fn get_logout(req: HttpRequest) -> Result<HttpResponse> {
    if let Some(c) = req.cookie(SESSION_ID) {
        match Session::try_from(c.value()) {
            Ok(session) => {
                if let Some(refresh_token) = session.refresh_token {
                    let _ = auth_client().do_revoke_token(&refresh_token).await;
                }
            }
            Err(_) => {}
        }
    }
    Ok(HttpResponse::SeeOther()
        .cookie(
            Cookie::build(SESSION_ID, "")
                .path(DEFAULT_INDEX)
                .expires(cookie::time::OffsetDateTime::now_utc())
                .same_site(cookie::SameSite::Lax)
                .secure(false)
                .http_only(false)
                .finish(),
        )
        .insert_header((header::LOCATION, "/auth/login"))
        .finish())
}

async fn get_session(req: HttpRequest) -> Result<HttpResponse> {
    if let Some(cookie) = req.cookie(SESSION_ID) {
        let session = extract_session(cookie.value())?;
        let dt = DateTime::<Utc>::from_utc(
            NaiveDateTime::from_timestamp_opt(session.expires, 0).unwrap(),
            Utc,
        );

        let props = serde_json::json!({
            "user": {
                "id": session.user_id,
                "name": session.email,
                "email": session.email,
                "image": session.picture,
                "picture": session.picture,
                "groups": [],
            },
            "expires" : dt.naive_utc(),
            "accessToken": session.access_token,
            "authProvider": "auth0"
        });

        return Ok(HttpResponse::Ok().json(props));
    }
    redirect_login()
}

async fn get_chat(
    req: HttpRequest,
    conversation_id: Option<web::Path<String>>,
    mut query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    if let Some(cookie) = req.cookie(SESSION_ID) {
        return match extract_session(cookie.value()) {
            Ok(session) => {
                let (template_name, path) = match conversation_id {
                    Some(conversation_id) => {
                        query.insert("chatId".to_string(), conversation_id.into_inner());
                        (TEMP_DETAIL, "/c/[chatId]")
                    }
                    None => (TEMP_CHAT, DEFAULT_INDEX),
                };
                let props = serde_json::json!({
                    "props": {
                        "pageProps": {
                            "user": {
                                "id": session.user_id,
                                "name": session.email,
                                "email": session.email,
                                "image": session.picture,
                                "picture": session.picture,
                                "groups": [],
                            },
                            "serviceStatus": {},
                            "userCountry": "US",
                            "geoOk": true,
                            "serviceAnnouncement": {
                                "paid": {},
                                "public": {}
                            },
                            "isUserInCanPayGroup": true
                        },
                        "__N_SSP": true
                    },
                    "page": path,
                    "query": query.into_inner(),
                    "buildId": BUILD_ID,
                    "isFallback": false,
                    "gssp": true,
                    "scriptLoader": []
                });
                let mut ctx = tera::Context::new();
                ctx.insert(
                    "props",
                    &serde_json::to_string(&props)
                        .map_err(|e| error::ErrorInternalServerError(e.to_string()))?,
                );
                settings_template_data(&mut ctx);
                return render_template(template_name, &ctx);
            }
            Err(_) => redirect_login(),
        };
    }
    redirect_login()
}

async fn get_chat_info(req: HttpRequest) -> Result<HttpResponse> {
    if let Some(cookie) = req.cookie(SESSION_ID) {
        return match extract_session(cookie.value()) {
            Ok(session) => {
                let body = serde_json::json!({
                    "pageProps": {
                        "user": {
                            "id": session.user_id,
                            "name": session.email,
                            "email": session.email,
                            "image": session.picture,
                            "picture": session.picture,
                            "groups": [],
                        },
                        "serviceStatus": {},
                        "userCountry": "US",
                        "geoOk": true,
                        "serviceAnnouncement": {
                            "paid": {},
                            "public": {}
                        },
                        "isUserInCanPayGroup": true
                    },
                    "__N_SSP": true
                });

                Ok(HttpResponse::Ok().json(body))
            }
            Err(_) => {
                let body = serde_json::json!(
                    {"pageProps": {"__N_REDIRECT": "/auth/login?", "__N_REDIRECT_STATUS": 307}, "__N_SSP": true}
                );
                Ok(HttpResponse::Ok().json(body))
            }
        };
    }
    redirect_login()
}

async fn get_share_chat(req: HttpRequest, share_id: web::Path<String>) -> Result<HttpResponse> {
    let share_id = share_id.into_inner();
    if let Some(cookie) = req.cookie(SESSION_ID) {
        return match extract_session(cookie.value()) {
            Ok(session) => {
                let url = get_url();
                let resp = super::api_client()
                    .get(format!("{url}/backend-api/share/{share_id}"))
                    .bearer_auth(session.access_token)
                    .send()
                    .await
                    .map_err(|e| error::ErrorInternalServerError(e.to_string()))?;

                match resp.json::<Value>().await {
                    Ok(mut share_data) => {
                        if let Some(replace) = share_data
                            .get_mut("continue_conversation_url")
                            .and_then(|v| v.as_str())
                        {
                            let new_value = replace.replace("https://chat.openai.com", "");
                            share_data.as_object_mut().and_then(|data| {
                                data.insert(
                                    "continue_conversation_url".to_owned(),
                                    json!(new_value),
                                )
                            });
                        }

                        let props = serde_json::json!({
                                    "props": {
                                        "pageProps": {
                                            "sharedConversationId": share_id,
                                            "serverResponse": {
                                                "type": "data",
                                                "data": share_data
                                            },
                                            "continueMode": false,
                                            "moderationMode": false,
                                            "chatPageProps": {},
                                        },
                                        "__N_SSP": true
                                    },
                                    "page": "/share/[[...shareParams]]",
                                    "query": {
                                        "shareParams": vec![share_id]
                                    },
                                    "buildId": BUILD_ID,
                                    "isFallback": false,
                                    "gssp": true,
                                    "scriptLoader": []
                                }
                        );
                        let mut ctx = tera::Context::new();
                        ctx.insert("props", &props.to_string());
                        settings_template_data(&mut ctx);
                        render_template(TEMP_SHARE, &ctx)
                    }
                    Err(_) => {
                        let props = serde_json::json!({
                            "props": {
                                "pageProps": {"statusCode": 404}
                            },
                            "page": "/_error",
                            "query": {},
                            "buildId": BUILD_ID,
                            "nextExport": true,
                            "isFallback": false,
                            "gip": true,
                            "scriptLoader": []
                        });

                        let mut ctx = tera::Context::new();
                        ctx.insert("props", &props.to_string());
                        settings_template_data(&mut ctx);
                        render_template(TEMP_404, &ctx)
                    }
                }
            }
            Err(_) => Ok(HttpResponse::Found()
                .insert_header((
                    header::LOCATION,
                    format!("/auth/login?next=%2Fshare%2F{share_id}"),
                ))
                .finish()),
        };
    }

    redirect_login()
}

async fn get_share_chat_info(
    req: HttpRequest,
    share_id: web::Path<String>,
) -> Result<HttpResponse> {
    let share_id = share_id.into_inner();
    if let Some(cookie) = req.cookie(SESSION_ID) {
        if let Ok(session) = extract_session(cookie.value()) {
            let url = get_url();
            let resp = super::api_client()
                .get(format!("{url}/backend-api/share/{share_id}"))
                .bearer_auth(session.access_token)
                .send()
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?;

            return match resp.json::<Value>().await {
                Ok(mut share_data) => {
                    if let Some(replace) = share_data
                        .get_mut("continue_conversation_url")
                        .and_then(|v| v.as_str())
                    {
                        let new_value = replace.replace("https://chat.openai.com", "");
                        share_data.as_object_mut().and_then(|data| {
                            data.insert("continue_conversation_url".to_owned(), json!(new_value))
                        });
                    }

                    let props = serde_json::json!({
                        "pageProps": {
                            "sharedConversationId": share_id,
                            "serverResponse": {
                                "type": "data",
                                "data": share_data,
                            },
                            "continueMode": false,
                            "moderationMode": false,
                            "chatPageProps": {},
                        },
                        "__N_SSP": true
                    }
                    );
                    Ok(HttpResponse::Ok().json(props))
                }
                Err(_) => Ok(HttpResponse::Ok().json(serde_json::json!({"notFound": true}))),
            };
        }

        return Ok(HttpResponse::Found()
            .insert_header((
                header::LOCATION,
                format!("/auth/login?next=%2Fshare%2F{share_id}"),
            ))
            .finish());
    }
    redirect_login()
}

async fn get_share_chat_continue(share_id: web::Path<String>) -> Result<HttpResponse> {
    Ok(HttpResponse::PermanentRedirect()
        .insert_header((
            header::LOCATION,
            format!("/share/{}", share_id.into_inner()),
        ))
        .finish())
}

async fn get_share_chat_continue_info(
    req: HttpRequest,
    share_id: web::Path<String>,
) -> Result<HttpResponse> {
    if let Some(cookie) = req.cookie(SESSION_ID) {
        return match extract_session(cookie.value()) {
            Ok(session) => {
                let url = get_url();
                let resp = super::api_client()
                .get(format!("{url}/backend-api/share/{share_id}"))
                .bearer_auth(session.access_token)
                .send()
                .await
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))?;
            match resp.json::<Value>().await {
                Ok(mut share_data) => {
                    if let Some(replace) = share_data
                        .get_mut("continue_conversation_url")
                        .and_then(|v| v.as_str())
                    {
                        let new_value = replace.replace("https://chat.openai.com", "");
                        share_data.as_object_mut().and_then(|data| {
                            data.insert("continue_conversation_url".to_owned(), json!(new_value))
                        });
                    }
                    let props = serde_json::json!({
                        "pageProps": {
                            "user": {
                                "id": session.user_id,
                                "name": session.email,
                                "email": session.email,
                                "image": session.picture,
                                "picture": session.picture,
                                "groups": [],
                            },
                            "serviceStatus": {},
                            "userCountry": "US",
                            "geoOk": true,
                            "serviceAnnouncement": {
                                "paid": {},
                                "public": {}
                            },
                            "isUserInCanPayGroup": true,
                            "sharedConversationId": share_id.into_inner(),
                            "serverResponse": {
                                "type": "data",
                                "data": share_data,
                            },
                            "continueMode": true,
                            "moderationMode": false,
                            "chatPageProps": {
                                "user": {
                                    "id": session.user_id,
                                    "name": session.email,
                                    "email": session.email,
                                    "image": session.picture,
                                    "picture": session.picture,
                                    "groups": [],
                                },
                                "serviceStatus": {},
                                "userCountry": "US",
                                "geoOk": true,
                                "serviceAnnouncement": {
                                    "paid": {},
                                    "public": {}
                                },
                                "isUserInCanPayGroup": true,
                            },
                        },
                        "__N_SSP": true
                    });
                    Ok(HttpResponse::Ok().json(props))
                }
                Err(_) => Ok(HttpResponse::Ok()
                    .append_header(("referrer-policy", "same-origin"))
                    .json(serde_json::json!({"notFound": true}))),
            }
            },
            Err(_) => {
                Ok(HttpResponse::TemporaryRedirect()
                .json(serde_json::json!({
                    "pageProps": {
                        "__N_REDIRECT": format!("/auth/login?next=%2Fshare%2F{}%2Fcontinue", share_id.into_inner()),
                        "__N_REDIRECT_STATUS": 307
                    },
                    "__N_SSP": true
                })))
            },
        };
    }
    redirect_login()
}

async fn get_image(params: Option<web::Query<ImageQuery>>) -> Result<HttpResponse> {
    let query = params.ok_or(error::ErrorBadRequest("Missing URL parameter"))?;
    let resp = super::api_client().get(&query.url).send().await;
    Ok(super::response_convert(resp))
}

async fn get_error_404() -> Result<HttpResponse> {
    let mut ctx = tera::Context::new();
    let props = json!(
        {
            "props": {
                "pageProps": {"statusCode": 404}
            },
            "page": "/_error",
            "query": {},
            "buildId": BUILD_ID,
            "nextExport": true,
            "isFallback": false,
            "gip": false,
            "scriptLoader": []
        }
    );
    ctx.insert(
        "props",
        &serde_json::to_string(&props)
            .map_err(|e| error::ErrorInternalServerError(e.to_string()))?,
    );
    render_template(TEMP_404, &ctx)
}

fn extract_session(cookie_value: &str) -> Result<Session> {
    Session::try_from(cookie_value)
        .map_err(|_| error::ErrorUnauthorized("invalid session"))
        .and_then(|session| match check_token(&session.access_token) {
            Ok(_) => Ok(session),
            Err(err) => Err(err),
        })
}

fn redirect_login() -> Result<HttpResponse> {
    Ok(HttpResponse::Found()
        .insert_header((header::LOCATION, "/auth/login"))
        .finish())
}

fn render_template(name: &str, context: &tera::Context) -> Result<HttpResponse> {
    let tm = unsafe {
        TEMPLATE
            .as_ref()
            .unwrap()
            .render(name, context)
            .map_err(|e| error::ErrorInternalServerError(e.to_string()))
    }?;
    Ok(HttpResponse::Ok()
        .content_type(header::ContentType::html())
        .body(tm))
}

fn settings_template_data(ctx: &mut tera::Context) {
    let data = unsafe { TEMPLATE_DATA.as_ref().unwrap() };
    if let Some(site_key) = &data.cf_site_key {
        ctx.insert("site_key", site_key);
    }
    if let Some(api_prefix) = &data.api_prefix {
        ctx.insert("api_prefix", api_prefix);
    }
}

fn check_token(token: &str) -> Result<()> {
    let _ = crate::token::check(token).map_err(|e| error::ErrorUnauthorized(e.to_string()))?;
    Ok(())
}

async fn cf_captcha_check(req: &HttpRequest, cf_response: Option<&str>) -> Result<()> {
    let data = unsafe { TEMPLATE_DATA.as_ref().unwrap() };
    if data.cf_site_key.is_some() && data.cf_secret_key.is_some() {
        return match cf_response {
            Some(cf_response) => {
                if cf_response.is_empty() {
                    return Err(error::ErrorBadRequest("Missing cf_captcha_response"));
                }

                let conn = req.connection_info();
                let form = CfCaptchaForm {
                    secret: data.cf_secret_key.as_ref().unwrap(),
                    response: cf_response,
                    remoteip: conn.peer_addr().unwrap(),
                    idempotency_key: crate::uuid::uuid(),
                };

                let resp = super::api_client()
                    .post("https://challenges.cloudflare.com/turnstile/v0/siteverify")
                    .form(&form)
                    .send()
                    .await
                    .map_err(|e| error::ErrorInternalServerError(e.to_string()))?;
                match resp.error_for_status() {
                    Ok(_) => Ok(()),
                    Err(e) => Err(error::ErrorUnauthorized(e.to_string())),
                }
            }
            None => Err(error::ErrorBadRequest("Missing cf_captcha_response")),
        };
    };
    Ok(())
}

async fn get_static_resource(path: web::Path<String>) -> impl Responder {
    let path = path.into_inner();
    let mut x = unsafe { STATIC_FILES.as_ref().unwrap().iter() };
    match x.find(|(k, _v)| k.contains(&path)) {
        Some((_, v)) => HttpResponse::Ok().content_type(v.mime_type).body(v.data),
        None => HttpResponse::NotFound().finish(),
    }
}

fn get_url() -> &'static str {
    let data = unsafe { TEMPLATE_DATA.as_ref().unwrap() };
    match data.api_prefix.as_ref() {
        Some(ref api_prefix) => api_prefix,
        None => URL_CHATGPT_API,
    }
}
#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct ImageQuery {
    url: String,
    w: String,
    q: String,
}

#[derive(serde::Serialize)]
struct CfCaptchaForm<'a> {
    secret: &'a str,
    response: &'a str,
    remoteip: &'a str,
    idempotency_key: String,
}
