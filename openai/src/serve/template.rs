use std::collections::HashMap;

use actix_web::{
    cookie::{self, Cookie},
    error,
    http::header,
    post, web, HttpRequest, HttpResponse, Responder, Result,
};
use chrono::prelude::{DateTime, Utc};
use chrono::NaiveDateTime;

use serde_json::{json, Value};

use crate::{auth, URL_CHATGPT_API};

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

const SESSION_ID: &str = "opengpt_session";
const BUILD_ID: &str = "WLHd8p-1ysAW_5sZZPJIy";
const TEMP_404: &str = "404.htm";
const TEMP_AUTH: &str = "auth.htm";
const TEMP_CHAT: &str = "chat.htm";
const TEMP_DETAIL: &str = "detail.htm";
const TEMP_LOGIN: &str = "login.htm";
const TEMP_SHARE: &str = "share.htm";

#[allow(dead_code)]
struct Session<'a> {
    token: &'a str,
    picture: &'a str,
}

#[allow(dead_code)]
#[derive(Clone)]
pub(super) struct TemplateData {
    pub(crate) api_prefix: String,
}

async fn get_static_resource(
    resource_map: web::Data<HashMap<&'static str, ::static_files::Resource>>,
    path: web::Path<String>,
) -> impl Responder {
    let path = path.into_inner();
    match resource_map.iter().find(|(k, _v)| k.contains(&path)) {
        Some((_, v)) => HttpResponse::Ok().content_type(v.mime_type).body(v.data),
        None => HttpResponse::NotFound().finish(),
    }
}

// this function could be located in a different module
pub fn config(cfg: &mut web::ServiceConfig) {
    let mut tera = tera::Tera::default();
    tera.add_raw_templates(vec![
        (TEMP_404, include_str!("../../templates/404.htm")),
        (TEMP_AUTH, include_str!("../../templates/auth.htm")),
        (TEMP_LOGIN, include_str!("../../templates/login.htm")),
        (TEMP_CHAT, include_str!("../../templates/chat.htm")),
        (TEMP_DETAIL, include_str!("../../templates/detail.htm")),
        (TEMP_SHARE, include_str!("../../templates/share.htm")),
    ])
    .expect("The static template failed to load");
    cfg.app_data(web::Data::new(tera))
        .app_data(web::Data::new(generate()))
        .route("/auth", web::get().to(get_auth))
        .route("/login", web::get().to(get_login))
        .route("/login", web::post().to(post_login))
        .route("/auth/logout", web::get().to(get_logout))
        .route("/api/auth/session", web::get().to(get_session))
        .route("/", web::get().to(get_chat))
        .route("/c", web::get().to(get_chat))
        .route("/c/{conversation_id}", web::get().to(get_chat))
        .service(web::redirect("/chat", "/"))
        .service(web::redirect("/chat/{conversation_id}", "/"))
        .route("/share/{share_id}", web::get().to(get_share_chat))
        .route(
            "/_next/data/WLHd8p-1ysAW_5sZZPJIy/index.json",
            web::get().to(get_chat_info),
        )
        .route(
            "/_next/data/WLHd8p-1ysAW_5sZZPJIy/c/{conversation_id}.json",
            web::get().to(get_chat_info),
        )
        .route(
            "/share/{share_id}/continue",
            web::get().to(get_share_chat_continue),
        )
        .route(
            "/_next/data/WLHd8p-1ysAW_5sZZPJIy/share/{share_id}.json",
            web::get().to(get_share_chat_info),
        )
        .route(
            "/_next/data/WLHd8p-1ysAW_5sZZPJIy/share/{share_id}/continue.json",
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

async fn get_auth(tmpl: web::Data<tera::Tera>) -> Result<HttpResponse> {
    render_template(tmpl, TEMP_SHARE, &tera::Context::new())
}

async fn get_login(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    let mut ctx = tera::Context::new();
    ctx.insert("next", query.get("next").unwrap_or(&"".to_owned()));
    ctx.insert("error", "");
    ctx.insert("username", "");
    render_template(tmpl, TEMP_LOGIN, &ctx)
}

async fn post_login(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
    account: web::Form<auth::OAuthAccount>,
) -> impl Responder {
    let default_next = "/".to_owned();
    let next = query.get("next").unwrap_or(&default_next);
    let account = account.into_inner();
    match super::auth_client().do_access_token(&account).await {
        Ok(access_token) => HttpResponse::SeeOther()
            .cookie(
                Cookie::build(SESSION_ID, access_token.access_token)
                    .path(next)
                    .max_age(cookie::time::Duration::seconds(access_token.expires_in))
                    .same_site(cookie::SameSite::Lax)
                    .secure(false)
                    .http_only(true)
                    .finish(),
            )
            .append_header((header::LOCATION, next.to_owned()))
            .finish(),
        Err(e) => {
            let mut ctx = tera::Context::new();
            ctx.insert("next", next.as_str());
            ctx.insert("username", account.username());
            ctx.insert("error", &e.to_string());
            render_template(tmpl, TEMP_LOGIN, &ctx).unwrap()
        }
    }
}

#[post("/login/token")]
async fn login_token(req: HttpRequest) -> Result<HttpResponse> {
    match req.headers().get(header::AUTHORIZATION) {
        Some(token) => {
            match crate::token::verify_access_token(token.to_str().unwrap_or_default()).await {
                Ok(token_profile) => {
                    let profile = token_profile
                        .ok_or(error::ErrorInternalServerError("Get Profile Erorr"))?;
                    Ok(HttpResponse::SeeOther()
                        .insert_header((header::LOCATION, "/"))
                        .cookie(
                            Cookie::build(SESSION_ID, token.to_str().unwrap())
                                .path("/")
                                .max_age(cookie::time::Duration::seconds(profile.expires()))
                                .same_site(cookie::SameSite::Lax)
                                .secure(false)
                                .http_only(true)
                                .finish(),
                        )
                        .finish())
                }
                Err(e) => Ok(HttpResponse::BadRequest().json(e.to_string())),
            }
        }
        None => redirect_login(),
    }
}

async fn get_logout(req: HttpRequest) -> impl Responder {
    match req.cookie(SESSION_ID) {
        Some(cookie) => {
            let _ = super::auth_client().do_revoke_token(cookie.value()).await;
        }
        None => {}
    }
    HttpResponse::SeeOther()
        .cookie(
            Cookie::build(SESSION_ID, "")
                .path("/")
                .secure(false)
                .http_only(true)
                .finish(),
        )
        .insert_header((header::LOCATION, "/login"))
        .finish()
}

async fn get_session(req: HttpRequest) -> Result<HttpResponse> {
    match req.cookie(SESSION_ID) {
        Some(cookie) => match crate::token::verify_access_token(cookie.value()).await {
            Ok(token_profile) => {
                let profile =
                    token_profile.ok_or(error::ErrorInternalServerError("Get Profile Erorr"))?;

                let dt = DateTime::<Utc>::from_utc(
                    NaiveDateTime::from_timestamp_opt(profile.expires_at(), 0).unwrap(),
                    Utc,
                );

                let props = serde_json::json!({
                    "user": {
                        "id": profile.user_id(),
                        "name": profile.email(),
                        "email": profile.email(),
                        "image": "",
                        "picture": "",
                        "groups": [],
                    },
                    "expires" : dt.naive_utc(),
                    "accessToken": cookie.value(),
                    "authProvider": "auth0"
                });

                Ok(HttpResponse::Ok().json(props))
            }
            Err(_) => redirect_login(),
        },
        None => redirect_login(),
    }
}

async fn get_chat(
    tmpl: web::Data<tera::Tera>,
    req: HttpRequest,
    conversation_id: Option<web::Path<String>>,
    mut query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse> {
    match req.cookie(SESSION_ID) {
        Some(cookie) => match crate::token::verify_access_token(cookie.value()).await {
            Ok(token_profile) => {
                let profile =
                    token_profile.ok_or(error::ErrorInternalServerError("Get Profile Error"))?;
                let (template_name, path) = match conversation_id {
                    Some(conversation_id) => {
                        query.insert("chatId".to_string(), conversation_id.into_inner());
                        (TEMP_DETAIL, "/c/[chatId]")
                    }
                    None => (TEMP_CHAT, "/"),
                };
                let props = serde_json::json!({
                    "props": {
                        "pageProps": {
                            "user": {
                                "id": profile.user_id(),
                                "name": profile.email(),
                                "email": profile.email(),
                                "image": None::<String>,
                                "picture": None::<String>,
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
                render_template(tmpl, template_name, &ctx)
            }
            Err(_) => redirect_login(),
        },
        None => redirect_login(),
    }
}

async fn get_chat_info(req: HttpRequest) -> Result<HttpResponse> {
    match req.cookie(SESSION_ID) {
        Some(cookie) => match crate::token::verify_access_token(cookie.value()).await {
            Ok(token_profile) => {
                let profile =
                    token_profile.ok_or(error::ErrorInternalServerError("Get Profile Erorr"))?;
                let body = serde_json::json!({
                    "pageProps": {
                        "user": {
                            "id": profile.user_id(),
                            "name": profile.email(),
                            "email": profile.email(),
                            "image": "",
                            "picture": "",
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
        },
        None => redirect_login(),
    }
}

async fn get_share_chat(
    tmpl: web::Data<tera::Tera>,
    req: HttpRequest,
    share_id: web::Path<String>,
) -> Result<HttpResponse> {
    let share_id = share_id.into_inner();
    match req.cookie(SESSION_ID) {
        Some(cookie) => match crate::token::verify_access_token(cookie.value()).await {
            Ok(_) => {
                let resp = super::client()
                    .get(format!("{URL_CHATGPT_API}/backend-api/share/{share_id}"))
                    .bearer_auth(cookie.value())
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
                        render_template(tmpl, TEMP_SHARE, &ctx)
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
                        render_template(tmpl, TEMP_404, &ctx)
                    }
                }
            }
            Err(_) => Ok(HttpResponse::Found()
                .insert_header((
                    header::LOCATION,
                    format!("/login?next=%2Fshare%2F{share_id}"),
                ))
                .finish()),
        },
        None => redirect_login(),
    }
}

async fn get_share_chat_info(
    req: HttpRequest,
    share_id: web::Path<String>,
) -> Result<HttpResponse> {
    let share_id = share_id.into_inner();
    match req.cookie(SESSION_ID) {
        Some(cookie) => match crate::token::verify_access_token(cookie.value()).await {
            Ok(_) => {
                let resp = super::client()
                    .get(format!("{URL_CHATGPT_API}/backend-api/share/{share_id}"))
                    .bearer_auth(cookie.value())
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
                }
            }
            Err(_) => Ok(HttpResponse::Found()
                .insert_header((
                    header::LOCATION,
                    format!("/login?next=%2Fshare%2F{share_id}"),
                ))
                .finish()),
        },
        None => redirect_login(),
    }
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
    match req.cookie(SESSION_ID) {
        Some(cookie) => match crate::token::verify_access_token(cookie.value()).await {
            Ok(token_profile) => {
                let profile =
                    token_profile.ok_or(error::ErrorInternalServerError("Get Profile Erorr"))?;
                    let resp = super::client()
                    .get(format!("{URL_CHATGPT_API}/backend-api/share/{share_id}"))
                    .bearer_auth(cookie.value())
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
                                "pageProps": {
                                    "user": {
                                        "id": profile.user_id(),
                                        "name": profile.email(),
                                        "email": profile.email(),
                                        "image": None::<String>,
                                        "picture": None::<String>,
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
                                            "id": profile.user_id(),
                                            "name": profile.email(),
                                            "email": profile.email(),
                                            "image": None::<String>,
                                            "picture": None::<String>,
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
            }
            Err(_) => {
                Ok(HttpResponse::TemporaryRedirect()
                .json(serde_json::json!({
                    "pageProps": {
                        "__N_REDIRECT": format!("/auth/login?next=%2Fshare%2F{}%2Fcontinue", share_id.into_inner()),
                        "__N_REDIRECT_STATUS": 307
                    },
                    "__N_SSP": true
                })))
            }
        },
        None => {
            redirect_login()
        },
    }
}

async fn get_image(params: Option<web::Query<ImageQuery>>) -> Result<HttpResponse> {
    let query = params.ok_or(error::ErrorBadRequest("Missing URL parameter"))?;
    let resp = super::client().get(&query.url).send().await;
    Ok(super::response_handle(resp))
}

async fn get_error_404(tmpl: web::Data<tera::Tera>) -> Result<HttpResponse> {
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
    render_template(tmpl, TEMP_404, &ctx)
}

fn redirect_login() -> Result<HttpResponse> {
    Ok(HttpResponse::Found()
        .insert_header((header::LOCATION, "/login"))
        .finish())
}

fn render_template(
    tmpl: web::Data<tera::Tera>,
    template_name: &str,
    context: &tera::Context,
) -> Result<HttpResponse> {
    let tm = tmpl
        .render(template_name, context)
        .map_err(|e| error::ErrorInternalServerError(e.to_string()))?;
    Ok(HttpResponse::Ok()
        .content_type(header::ContentType::html())
        .body(tm))
}

#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct ImageQuery {
    url: String,
    w: String,
    q: String,
}
