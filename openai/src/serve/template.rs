use std::collections::HashMap;

use actix_web::{
    cookie::{self, Cookie},
    error, get,
    http::header,
    post, web, HttpRequest, HttpResponse, Responder,
};
use chrono::TimeZone;
use serde_json::json;

use crate::auth;
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

const SESSION_ID: &str = "opengpt_session";
const BUILD_ID: &str = "cx416mT2Lb0ZTj5FxFg1l";

#[allow(dead_code)]
struct Session<'a> {
    token: &'a str,
    picture: &'a str,
}

async fn static_service(
    resource_map: web::Data<HashMap<&'static str, ::static_files::Resource>>,
    path: web::Path<String>,
) -> impl Responder {
    let path = path.into_inner();
    println!("{}", path);
    match resource_map.iter().find(|(k, _v)| k.contains(&path)) {
        Some((_, v)) => HttpResponse::Ok().content_type(v.mime_type).body(v.data),
        None => HttpResponse::NotFound().finish(),
    }
}

// this function could be located in a different module
pub fn config(cfg: &mut web::ServiceConfig) {
    let tera = tera::Tera::new(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/*.html")).unwrap();
    cfg.app_data(web::Data::new(tera))
        .app_data(web::Data::new(generate()))
        .service(get_auth)
        .service(get_login)
        .service(post_login)
        .service(get_logout)
        .service(get_chat)
        .service(get_chat_conversation)
        .service(get_session)
        .service(get_account_check)
        .service(e404)
        // static resource endpoints
        .service(
            web::resource("/{filename:.+\\.(png|js|css|webp|json)}")
                .route(web::get().to(static_service)),
        )
        .service(web::resource("/_next/{tail:.*}").route(web::to(static_service)))
        .service(web::resource("/fonts/{tail:.*}").route(web::to(static_service)))
        .service(web::resource("/ulp/{tail:.*}").route(web::to(static_service)))
        .service(web::resource("/sweetalert2/{tail:.*}").route(web::to(static_service)));
}

#[get("/auth")]
async fn get_auth(tmpl: web::Data<tera::Tera>) -> impl Responder {
    let tm = tmpl
        .render("auth.html", &tera::Context::new())
        .map_err(|e| error::ErrorInternalServerError(e.to_string()))
        .unwrap();
    HttpResponse::Ok()
        .content_type(header::ContentType::html())
        .body(tm)
}

#[get("/")]
async fn get_chat(tmpl: web::Data<tera::Tera>, req: HttpRequest) -> impl Responder {
    match req.cookie(SESSION_ID) {
        Some(cookie) => match crate::token::verify_access_token(cookie.value()).await {
            Ok(token_profile) => match token_profile {
                Some(profile) => {
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
                        "page": "/",
                        "query": req.query_string(),
                        "buildId": BUILD_ID,
                        "isFallback": false,
                        "gssp": true,
                        "scriptLoader": []
                    });
                    let mut ctx = tera::Context::new();
                    ctx.insert("props", &serde_json::to_string(&props).unwrap());
                    let tm = tmpl
                        .render("chat.html", &ctx)
                        .map_err(|e| error::ErrorInternalServerError(e.to_string()))
                        .unwrap();
                    HttpResponse::Ok()
                        .content_type(header::ContentType::html())
                        .body(tm)
                }
                None => HttpResponse::InternalServerError().finish(),
            },
            Err(_) => redirect_login(),
        },
        None => redirect_login(),
    }
}

#[get("/c/{conversation_id}")]
async fn get_chat_conversation(
    tmpl: web::Data<tera::Tera>,
    req: HttpRequest,
    mut query: web::Query<HashMap<String, String>>,
    conversation_id: web::Path<String>,
) -> impl Responder {
    match req.cookie(SESSION_ID) {
        Some(cookie) => match crate::token::verify_access_token(cookie.value()).await {
            Ok(token_profile) => match token_profile {
                Some(profile) => {
                    let conversation_id = conversation_id.into_inner();
                    query.insert("chatId".to_string(), conversation_id.clone());
                    let props = serde_json::json!({
                        "props": {
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
                        },
                        "page": "/c/[chatId]",
                        "query": hashmap_to_query_string(&query.into_inner()),
                        "buildId": BUILD_ID,
                        "isFallback": false,
                        "gssp": true,
                        "scriptLoader": []
                    });

                    let mut ctx = tera::Context::new();
                    ctx.insert("props", &props.to_string());
                    let tm = tmpl
                        .render("detail.html", &ctx)
                        .map_err(|e| error::ErrorInternalServerError(e.to_string()))
                        .unwrap();
                    HttpResponse::Ok()
                        .content_type(header::ContentType::html())
                        .body(tm)
                }
                None => HttpResponse::InternalServerError().finish(),
            },
            Err(_) => redirect_login(),
        },
        None => redirect_login(),
    }
}

#[get("/login")]
async fn get_login(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let mut ctx = tera::Context::new();
    ctx.insert("next", query.get("next").unwrap_or(&"".to_owned()));
    ctx.insert("error", "");
    ctx.insert("username", "");
    let tm = tmpl
        .render("login.html", &ctx)
        .map_err(|e| error::ErrorInternalServerError(e.to_string()))
        .unwrap();
    HttpResponse::Ok()
        .content_type(header::ContentType::html())
        .body(tm)
}

#[post("/login")]
async fn post_login(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
    account: web::Form<auth::OAuthAccount>,
) -> impl Responder {
    let default_next = "/".to_owned();
    let next = query.get("next").unwrap_or(&default_next);
    let account = account.into_inner();
    match super::oauth_client()
        .do_access_token(&account)
        .await
    {
        Ok(access_token) => HttpResponse::Found()
            .append_header((header::LOCATION, next.as_str()))
            .cookie(
                Cookie::build(SESSION_ID, access_token.access_token)
                    .path("/")
                    .max_age(cookie::time::Duration::seconds(access_token.expires_in))
                    .same_site(cookie::SameSite::Lax)
                    .secure(false)
                    .http_only(true)
                    .finish(),
            )
            .finish(),
        Err(e) => {
            let mut ctx = tera::Context::new();
            ctx.insert("next", next.as_str());
            ctx.insert("username", account.username());
            let tm = tmpl
                .render("login.html", &ctx)
                .map_err(|e| error::ErrorInternalServerError(e.to_string()))
                .unwrap();
            ctx.insert("error", &e.to_string());
            HttpResponse::Ok()
                .content_type(header::ContentType::html())
                .body(tm)
        }
    }
}

#[post("/login/token")]
async fn login_token(req: HttpRequest) -> impl Responder {
    match req.headers().get(header::AUTHORIZATION) {
        Some(token) => {
            match crate::token::verify_access_token(token.to_str().unwrap_or_default()).await {
                Ok(token_profile) => match token_profile {
                    Some(profile) => HttpResponse::Found()
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
                        .finish(),
                    None => HttpResponse::InternalServerError().finish(),
                },
                Err(e) => HttpResponse::BadRequest().json(e.to_string()),
            }
        }
        None => redirect_login(),
    }
}

#[get("/auth/logout")]
async fn get_logout(req: HttpRequest) -> impl Responder {
    match req.cookie(SESSION_ID) {
        Some(cookie) => {
            let _ = super::oauth_client().do_revoke_token(cookie.value()).await;
        }
        None => {}
    }
    HttpResponse::Found()
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

#[get("/api/auth/session")]
async fn get_session(req: HttpRequest) -> impl Responder {
    match req.cookie(SESSION_ID) {
        Some(cookie) => match crate::token::verify_access_token(cookie.value()).await {
            Ok(token_profile) => match token_profile {
                Some(profile) => {
                    let time = chrono::Utc
                        .timestamp_millis_opt(profile.expires_at())
                        .unwrap();
                    let props = serde_json::json!({
                        "user": {
                            "id": profile.user_id(),
                            "name": profile.email(),
                            "email": profile.email(),
                            "image": None::<String>,
                            "picture": None::<String>,
                            "groups": [],
                        },
                        "expires" : time.format("%Y-%m-%d %H:%M:%S").to_string(),
                        "accessToken": cookie.value(),
                        "authProvider": "auth0"
                    });

                    HttpResponse::Ok().json(props)
                }
                None => HttpResponse::InternalServerError().finish(),
            },
            Err(_) => redirect_login(),
        },
        None => redirect_login(),
    }
}

#[get("/api/accounts/check/v4-2023-04-27")]
async fn get_account_check() -> impl Responder {
    let res = serde_json::json!({
        "accounts": {
            "default": {
                "account": {
                    "account_user_role": "account-owner",
                    "account_user_id": "d0322341-7ace-4484-b3f7-89b03e82b927",
                    "processor": {
                        "a001": {
                            "has_customer_object": true
                        },
                        "b001": {
                            "has_transaction_history": true
                        }
                    },
                    "account_id": "a323bd05-db25-4e8f-9173-2f0c228cc8fa",
                    "is_most_recent_expired_subscription_gratis": true,
                    "has_previously_paid_subscription": true
                },
                "features": [
                    "model_switcher",
                    "model_preview",
                    "system_message",
                    "data_controls_enabled",
                    "data_export_enabled",
                    "show_existing_user_age_confirmation_modal",
                    "bucketed_history",
                    "priority_driven_models_list",
                    "message_style_202305",
                    "layout_may_2023",
                    "plugins_available",
                    "beta_features",
                    "infinite_scroll_history",
                    "browsing_available",
                    "browsing_inner_monologue",
                    "browsing_bing_branding",
                    "shareable_links",
                    "plugin_display_params",
                    "tools3_dev",
                    "tools2",
                    "debug",
                ],
                "entitlement": {
                    "subscription_id": "d0dcb1fc-56aa-4cd9-90ef-37f1e03576d3",
                    "has_active_subscription": true,
                    "subscription_plan": "chatgptplusplan",
                    "expires_at": "2089-08-08T23:59:59+00:00"
                },
                "last_active_subscription": {
                    "subscription_id": "d0dcb1fc-56aa-4cd9-90ef-37f1e03576d3",
                    "purchase_origin_platform": "chatgpt_mobile_ios",
                    "will_renew": true
                }
            }
        },
        "temp_ap_available_at": "2023-05-20T17:30:00+00:00"
    });
    HttpResponse::Ok().json(res)
}

#[get("/404")]
async fn e404(tmpl: web::Data<tera::Tera>) -> impl Responder {
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
    ctx.insert("props", &props);
    let tm = tmpl
        .render("404.html", &ctx)
        .map_err(|e| error::ErrorInternalServerError(e.to_string()))
        .unwrap();
    HttpResponse::Ok()
        .content_type(header::ContentType::html())
        .body(tm)
}

fn redirect_login() -> HttpResponse {
    HttpResponse::Found()
        .insert_header((header::LOCATION, "/login"))
        .finish()
}

fn hashmap_to_query_string(params: &std::collections::HashMap<String, String>) -> String {
    let mut query_string = String::new();

    for (key, value) in params {
        if !query_string.is_empty() {
            query_string.push('&');
        }
        query_string.push_str(&format!("{}={}", url_encode(key), url_encode(value)));
    }

    query_string
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::new();

    for ch in input.chars() {
        match ch {
            'A'..='Z'
            | 'a'..='z'
            | '0'..='9'
            | '-'
            | '_'
            | '.'
            | '!'
            | '~'
            | '*'
            | '\''
            | '('
            | ')' => {
                encoded.push(ch);
            }
            _ => {
                encoded.push('%');
                encoded.push_str(&ch.encode_utf8(&mut [0; 4]).to_string());
            }
        }
    }

    encoded
}
