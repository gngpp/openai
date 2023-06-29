use std::collections::HashMap;

use actix_web::{
    cookie::{self, Cookie},
    error, get,
    http::header,
    post, web, HttpRequest, HttpResponse, Responder,
};
use serde_json::json;

use crate::auth;
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

const BUILD_ID: &str = "cx416mT2Lb0ZTj5FxFg1l";

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
        .service(auth0_index)
        .service(login_index)
        .service(login_index_post)
        .service(chat_index)
        .service(e404_index)
        // static resource endpoints
        .service(
            web::resource("/{filename:.+\\.(png|js|css|webp|json)}")
                .route(web::get().to(static_service)),
        )
        .service(chat_conversation_index)
        .service(web::resource("/_next/{tail:.*}").route(web::to(static_service)))
        .service(web::resource("/fonts/{tail:.*}").route(web::to(static_service)))
        .service(web::resource("/ulp/{tail:.*}").route(web::to(static_service)))
        .service(web::resource("/sweetalert2/{tail:.*}").route(web::to(static_service)));
}

#[get("/auth0")]
async fn auth0_index(tmpl: web::Data<tera::Tera>) -> impl Responder {
    let tm = tmpl
        .render("auth0.html", &tera::Context::new())
        .map_err(|_| error::ErrorInternalServerError("Template error"))
        .unwrap();
    HttpResponse::Ok()
        .content_type(header::ContentType::html())
        .body(tm)
}

#[get("/")]
async fn chat_index(tmpl: web::Data<tera::Tera>, req: HttpRequest) -> impl Responder {
    match req.cookie("access-token") {
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
                        "page": "/",
                        "query": req.query_string(),
                        "buildId": BUILD_ID,
                        "isFallback": false,
                        "gssp": true,
                        "scriptLoader": []
                    });
                    let mut ctx = tera::Context::new();
                    ctx.insert("props", &props.to_string());
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
            Err(_) => unauthorized_redirect_login(),
        },
        None => redirect_login(),
    }
}

#[get("/{conversation_id}")]
async fn chat_conversation_index(
    tmpl: web::Data<tera::Tera>,
    req: HttpRequest,
    mut query: web::Query<HashMap<String, String>>,
    conversation_id: web::Path<String>,
) -> impl Responder {
    match req.cookie("access-token") {
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
                        "page": format!("/c/{}", conversation_id),
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
            Err(_) => unauthorized_redirect_login(),
        },
        None => redirect_login(),
    }
}

#[get("/login")]
async fn login_index(
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
async fn login_index_post(
    tmpl: web::Data<tera::Tera>,
    query: web::Query<HashMap<String, String>>,
    account: web::Form<auth::OAuthAccount>,
) -> impl Responder {
    let mut ctx = tera::Context::new();
    let default_next = "/".to_owned();
    let next = query.get("next").unwrap_or(&default_next);
    ctx.insert("next", next.as_str());
    ctx.insert("username", account.0.username());

    match super::oauth_client()
        .do_access_token(account.into_inner())
        .await
    {
        Ok(access_token) => HttpResponse::Found()
            .insert_header((header::LOCATION, next.as_str()))
            .content_type(header::ContentType::html())
            .cookie(
                Cookie::build("access-token", access_token.access_token)
                    .path("/")
                    .max_age(cookie::time::Duration::seconds(access_token.expires_in))
                    .secure(true)
                    .http_only(true)
                    .finish(),
            )
            .finish(),
        Err(e) => {
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

#[get("/404")]
async fn e404_index(tmpl: web::Data<tera::Tera>) -> impl Responder {
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

fn unauthorized_redirect_login() -> HttpResponse {
    HttpResponse::Unauthorized()
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
