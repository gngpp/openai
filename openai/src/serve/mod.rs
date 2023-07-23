pub mod middleware;
#[cfg(feature = "sign")]
pub mod sign;
#[cfg(feature = "limit")]
pub mod tokenbucket;

#[cfg(feature = "template")]
pub mod ui;

pub mod load_balancer;

use actix_web::cookie::{self, Cookie};
use actix_web::http::header;
use actix_web::middleware::Logger;
use actix_web::web::Json;
use actix_web::{get, post, App, HttpResponse, HttpServer, Responder};
use actix_web::{web, HttpRequest};

use derive_builder::Builder;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use serde_json::{json, Value};
use std::fs::File;
use std::io::BufReader;
use std::sync::Once;
use std::time::{Duration, UNIX_EPOCH};

use std::net::IpAddr;
use std::path::PathBuf;
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};

use crate::arkose::ArkoseToken;
use crate::auth::model::AuthAccount;
use crate::auth::{AuthClient, AuthHandle};
use crate::serve::tokenbucket::TokenBucketContext;
use crate::serve::ui::TemplateData;
use crate::{debug, info, warn, HOST_CHATGPT, ORIGIN_CHATGPT};

use crate::{HEADER_UA, URL_CHATGPT_API, URL_PLATFORM_API};
const EMPTY: &str = "";
static INIT: Once = Once::new();
static mut API_CLIENT: Option<load_balancer::ClientLoadBalancer<Client>> = None;
static mut AUTH_CLIENT: Option<load_balancer::ClientLoadBalancer<AuthClient>> = None;

pub(super) fn api_client() -> Client {
    if let Some(lb) = unsafe { &API_CLIENT } {
        return lb.next().clone();
    }
    panic!("The requesting client must be initialized")
}

pub(super) fn auth_client() -> AuthClient {
    if let Some(lb) = unsafe { &AUTH_CLIENT } {
        return lb.next().clone();
    }
    panic!("The requesting oauth client must be initialized")
}

#[derive(Builder, Clone)]
pub struct Launcher {
    /// Listen addres
    host: IpAddr,
    /// Listen port
    port: u16,
    /// Machine worker pool
    workers: usize,
    /// Server proxies
    proxies: Vec<String>,
    /// TCP keepalive (second)
    tcp_keepalive: usize,
    /// Client timeout
    timeout: usize,
    /// Client connect timeout
    connect_timeout: usize,
    /// TLS keypair
    tls_keypair: Option<(PathBuf, PathBuf)>,
    /// Web UI api prefix
    api_prefix: Option<String>,
    /// Enable url signature (signature secret key)
    #[cfg(feature = "sign")]
    sign_secret_key: Option<String>,
    /// Enable Tokenbucket
    #[cfg(feature = "limit")]
    tb_enable: bool,
    /// Tokenbucket store strategy
    #[cfg(feature = "limit")]
    tb_store_strategy: tokenbucket::Strategy,
    /// Tokenbucket redis url
    tb_redis_url: Vec<String>,
    /// Tokenbucket capacity
    #[cfg(feature = "limit")]
    tb_capacity: u32,
    /// Tokenbucket fill rate
    #[cfg(feature = "limit")]
    tb_fill_rate: u32,
    /// Tokenbucket expired (second)
    #[cfg(feature = "limit")]
    tb_expired: u32,
    /// Cloudflare turnstile captcha site key
    cf_site_key: Option<String>,
    /// Cloudflare turnstile captcha secret key
    cf_secret_key: Option<String>,
}

impl Launcher {
    pub async fn run(self) -> anyhow::Result<()> {
        // template data
        let template_data = TemplateData::from(self.clone());

        INIT.call_once(|| unsafe {
            API_CLIENT = Some(
                load_balancer::ClientLoadBalancer::<Client>::new_api_client(&self)
                    .expect("Failed to initialize the requesting client"),
            );
            AUTH_CLIENT = Some(
                load_balancer::ClientLoadBalancer::<AuthClient>::new_auth_client(&self)
                    .expect("Failed to initialize the requesting oauth client"),
            );
        });

        check_self_ip(&api_client()).await;

        info!(
            "Starting HTTP(S) server at http(s)://{}:{}",
            self.host, self.port
        );

        if let Some(url) = &self.api_prefix {
            info!("WebUI site use api: {url}")
        }

        // serve
        let serve = HttpServer::new(move || {
            let app = App::new()
                .wrap(
                    actix_cors::Cors::default()
                        .supports_credentials()
                        .allow_any_origin()
                        .allow_any_header()
                        .allow_any_method()
                        .max_age(3600),
                )
                .wrap(Logger::default())
                // official dashboard api endpoint
                .service(
                    web::resource("/dashboard/{tail:.*}")
                        .wrap(middleware::TokenAuthorization)
                        .route(web::to(official_proxy)),
                )
                // official v1 api endpoint
                .service(
                    web::resource("/v1/{tail:.*}")
                        .wrap(middleware::TokenAuthorization)
                        .route(web::to(official_proxy)),
                )
                // unofficial backend api endpoint
                .service(
                    web::resource("/backend-api/{tail:.*}")
                        .wrap(middleware::TokenAuthorization)
                        .route(web::to(unofficial_proxy)),
                )
                // unofficial public api endpoint
                .service(web::resource("/public-api/{tail:.*}").route(web::to(unofficial_proxy)))
                // auth endpoint
                .service(post_access_token)
                .service(post_refresh_token)
                .service(post_revoke_token)
                .service(get_arkose_token)
                // template data
                .app_data(web::Data::new(template_data.clone()))
                // templates page endpoint
                .configure(ui::config);

            #[cfg(all(not(feature = "sign"), feature = "limit"))]
            {
                return app.wrap(middleware::TokenBucketRateLimiter::new(
                    TokenBucketContext::from((
                        self.tb_store_strategy.clone(),
                        self.tb_enable,
                        self.tb_capacity,
                        self.tb_fill_rate,
                        self.tb_expired,
                    )),
                ));
            }

            #[cfg(all(feature = "sign", feature = "limit"))]
            {
                return app
                    .wrap(middleware::ApiSign::new(self.sign_secret_key.clone()))
                    .wrap(middleware::TokenBucketRateLimiter::new(
                        TokenBucketContext::from((
                            self.tb_store_strategy.clone(),
                            self.tb_enable,
                            self.tb_capacity,
                            self.tb_fill_rate,
                            self.tb_expired,
                            self.tb_redis_url.clone(),
                        )),
                    ));
            }

            #[cfg(all(not(feature = "limit"), feature = "sign"))]
            {
                return app.wrap(middleware::ApiSign::new(self.sign_secret_key.clone()));
            }

            #[cfg(not(any(feature = "sign", feature = "limit")))]
            app
        })
        .client_request_timeout(Duration::from_secs(self.timeout as u64))
        .tls_handshake_timeout(Duration::from_secs(self.connect_timeout as u64))
        .keep_alive(Duration::from_secs(self.tcp_keepalive as u64))
        .workers(self.workers);
        match self.tls_keypair {
            Some(keypair) => {
                let tls_config = Self::load_rustls_config(keypair.0, keypair.1).await?;
                serve
                    .bind_rustls((self.host, self.port), tls_config)?
                    .run()
                    .await
                    .map_err(|e| anyhow::anyhow!(e))
            }
            None => serve
                .bind((self.host, self.port))?
                .run()
                .await
                .map_err(|e| anyhow::anyhow!(e)),
        }
    }

    async fn load_rustls_config(
        tls_cert: PathBuf,
        tls_key: PathBuf,
    ) -> anyhow::Result<ServerConfig> {
        use rustls_pemfile::{certs, ec_private_keys, pkcs8_private_keys, rsa_private_keys};

        // init server config builder with safe defaults
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth();

        // load TLS key/cert files
        let cert_file = &mut BufReader::new(File::open(tls_cert)?);
        let key_file = &mut BufReader::new(File::open(tls_key)?);

        // convert files to key/cert objects
        let cert_chain = certs(cert_file)?.into_iter().map(Certificate).collect();

        let keys_list = vec![
            ec_private_keys(key_file)?,
            pkcs8_private_keys(key_file)?,
            rsa_private_keys(key_file)?,
        ];

        let keys = keys_list.into_iter().find(|k| !k.is_empty());

        // exit if no keys could be parsed
        match keys {
            Some(keys) => Ok(config.with_single_cert(
                cert_chain,
                keys.into_iter()
                    .map(PrivateKey)
                    .collect::<Vec<PrivateKey>>()
                    .remove(0),
            )?),
            None => anyhow::bail!("Could not locate EC/PKCS8/RSA private keys."),
        }
    }
}

#[post("/auth/token")]
async fn post_access_token(account: web::Form<AuthAccount>) -> impl Responder {
    match auth_client().do_access_token(&account.into_inner()).await {
        Ok(token) => HttpResponse::Ok().json(token),
        Err(err) => HttpResponse::BadRequest().json(err.to_string()),
    }
}

#[post("/auth/refresh_token")]
async fn post_refresh_token(req: HttpRequest) -> impl Responder {
    let refresh_token = req
        .headers()
        .get(header::AUTHORIZATION)
        .map_or(EMPTY, |e| e.to_str().unwrap_or_default());
    match auth_client().do_refresh_token(refresh_token).await {
        Ok(token) => HttpResponse::Ok().json(token),
        Err(err) => HttpResponse::BadRequest().json(err.to_string()),
    }
}

#[post("/auth/revoke_token")]
async fn post_revoke_token(req: HttpRequest) -> impl Responder {
    let refresh_token = req
        .headers()
        .get(header::AUTHORIZATION)
        .map_or(EMPTY, |e| e.to_str().unwrap_or_default());
    match auth_client().do_revoke_token(refresh_token).await {
        Ok(token) => HttpResponse::Ok().json(token),
        Err(err) => HttpResponse::BadRequest().json(err.to_string()),
    }
}

#[get("/auth/arkose_token")]
async fn get_arkose_token() -> impl Responder {
    match ArkoseToken::new("gpt4").await {
        Ok(arkose) => HttpResponse::Ok().json(arkose),
        Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
    }
}

/// match path /dashboard/{tail.*}
/// POST https://api.openai.com/dashboard/onboarding/login"
/// POST https://api.openai.com/dashboard/user/api_keys
/// GET https://api.openai.com/dashboard/user/api_keys
/// POST https://api.openai.com/dashboard/billing/usage
/// POST https://api.openai.com/dashboard/billing/credit_grants
///
/// platform API match path /v1/{tail.*}
/// reference: https://platform.openai.com/docs/api-reference
/// GET https://api.openai.com/v1/models
/// GET https://api.openai.com/v1/models/{model}
/// POST https://api.openai.com/v1/chat/completions
/// POST https://api.openai.com/v1/completions
/// POST https://api.openai.com/v1/edits
/// POST https://api.openai.com/v1/images/generations
/// POST https://api.openai.com/v1/images/edits
/// POST https://api.openai.com/v1/images/variations
/// POST https://api.openai.com/v1/embeddings
/// POST https://api.openai.com/v1/audio/transcriptions
/// POST https://api.openai.com/v1/audio/translations
/// GET https://api.openai.com/v1/files
/// POST https://api.openai.com/v1/files
/// DELETE https://api.openai.com/v1/files/{file_id}
/// GET https://api.openai.com/v1/files/{file_id}
/// GET https://api.openai.com/v1/files/{file_id}/content
/// POST https://api.openai.com/v1/fine-tunes
/// GET https://api.openai.com/v1/fine-tunes
/// GET https://api.openai.com/v1/fine-tunes/{fine_tune_id}
/// POST https://api.openai.com/v1/fine-tunes/{fine_tune_id}/cancel
/// GET https://api.openai.com/v1/fine-tunes/{fine_tune_id}/events
/// DELETE https://api.openai.com/v1/models/{model}
/// POST https://api.openai.com/v1/moderations
/// Deprecated GET https://api.openai.com/v1/engines
/// Deprecated GET https://api.openai.com/v1/engines/{engine_id}
async fn official_proxy(req: HttpRequest, body: Option<Json<Value>>) -> impl Responder {
    let url = if req.query_string().is_empty() {
        format!("{URL_PLATFORM_API}{}", req.path())
    } else {
        format!("{URL_PLATFORM_API}{}?{}", req.path(), req.query_string())
    };

    let builder = api_client()
        .request(req.method().clone(), url)
        .headers(header_convert(&req));
    let resp = match body {
        Some(body) => builder.json(&body).send().await,
        None => builder.send().await,
    };
    response_convert(resp)
}

/// reference: doc/http.rest
/// GET http://{{host}}/backend-api/models?history_and_training_disabled=false
/// GET http://{{host}}/backend-api/accounts/check
/// GET http://{{host}}/backend-api/accounts/check/v4-2023-04-27
/// GET http://{{host}}/backend-api/settings/beta_features
/// GET http://{{host}}/backend-api/conversation/{conversation_id}
/// GET http://{{host}}/backend-api/conversations?offset=0&limit=3&order=updated
/// GET http://{{host}}/public-api/conversation_limit
/// POST http://{{host}}/backend-api/conversation
/// PATCH http://{{host}}/backend-api/conversation/{conversation_id}
/// POST http://{{host}}/backend-api/conversation/gen_title/{conversation_id}
/// PATCH http://{{host}}/backend-api/conversation/{conversation_id}
/// PATCH http://{{host}}/backend-api/conversations
/// POST http://{{host}}/backend-api/conversation/message_feedback
async fn unofficial_proxy(req: HttpRequest, mut body: Option<Json<Value>>) -> impl Responder {
    gpt4_body_handle(&req, &mut body).await;

    let url = if req.query_string().is_empty() {
        format!("{URL_CHATGPT_API}{}", req.path())
    } else {
        format!("{URL_CHATGPT_API}{}?{}", req.path(), req.query_string())
    };

    let builder = api_client()
        .request(req.method().clone(), url)
        .headers(header_convert(&req));
    let resp = match body {
        Some(body) => builder.json(&body).send().await,
        None => builder.send().await,
    };
    response_convert(resp)
}

fn response_convert(resp: Result<reqwest::Response, reqwest::Error>) -> HttpResponse {
    match resp {
        Ok(resp) => {
            let status = resp.status();
            let mut builder = HttpResponse::build(status);
            resp.headers()
                .into_iter()
                .filter(|(k, _v)| {
                    let name = k.as_str().to_lowercase();
                    name.ne("__cf_bm")
                        || name.ne("__cfduid")
                        || name.ne("_cfuvid")
                        || name.ne("set-cookie")
                })
                .for_each(|kv| {
                    builder.insert_header(kv);
                });

            for c in resp
                .cookies()
                .into_iter()
                .filter(|c| c.name().eq("_puid") || c.name().eq("_account"))
            {
                if let Some(expires) = c.expires() {
                    let timestamp_nanos = expires
                        .duration_since(UNIX_EPOCH)
                        .expect("Failed to get timestamp")
                        .as_nanos() as i128;
                    let cookie = Cookie::build(c.name(), c.value())
                        .path("/")
                        .expires(
                            cookie::time::OffsetDateTime::from_unix_timestamp_nanos(
                                timestamp_nanos,
                            )
                            .expect("get cookie expires exception"),
                        )
                        .same_site(cookie::SameSite::Lax)
                        .secure(false)
                        .http_only(false)
                        .finish();
                    builder.cookie(cookie);
                }
            }
            builder.streaming(resp.bytes_stream())
        }
        Err(err) => HttpResponse::InternalServerError().json(err.to_string()),
    }
}

fn header_convert(req: &HttpRequest) -> reqwest::header::HeaderMap {
    let headers = req.headers();

    let authorization = match headers.get(header::AUTHORIZATION) {
        Some(v) => Some(v),
        // pandora will pass X-Authorization header
        None => headers.get("X-Authorization"),
    };

    let mut res = HeaderMap::new();
    if let Some(h) = authorization {
        res.insert(header::AUTHORIZATION, h.clone());
    }
    res.insert(header::HOST, HeaderValue::from_static(HOST_CHATGPT));
    res.insert(header::ORIGIN, HeaderValue::from_static(ORIGIN_CHATGPT));
    res.insert(header::USER_AGENT, HeaderValue::from_static(HEADER_UA));
    res.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    res.insert(
        "sec-ch-ua",
        HeaderValue::from_static(
            r#""Chromium";v="110", "Not A(Brand";v="24", "Google Chrome";v="110"#,
        ),
    );
    res.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
    res.insert("sec-ch-ua-platform", HeaderValue::from_static("Linux"));
    res.insert("sec-fetch-dest", HeaderValue::from_static("empty"));
    res.insert("sec-fetch-mode", HeaderValue::from_static("cors"));
    res.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
    res.insert("sec-gpc", HeaderValue::from_static("1"));
    res.insert("Pragma", HeaderValue::from_static("no-cache"));

    let mut cookie = String::new();

    if let Some(puid) = headers.get("PUID") {
        let puid = puid.to_str().unwrap();
        cookie.push_str(&format!("_puid={puid};"))
    }

    if let Some(cookier) = req.cookie("_puid") {
        let c = &format!("_puid={};", puid_cookie_encoded(cookier.value()));
        cookie.push_str(c);
        debug!("request cookie `puid`: {}", c);
    }

    // setting cookie
    if !cookie.is_empty() {
        res.insert(
            header::COOKIE,
            HeaderValue::from_str(cookie.as_str()).expect("setting cookie error"),
        );
    }
    res
}

fn puid_cookie_encoded(input: &str) -> String {
    let separator = ':';
    if let Some((name, value)) = input.split_once(separator) {
        let encoded_value = value
            .chars()
            .map(|ch| match ch {
                '!' | '#' | '$' | '%' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | '/' | ':'
                | ';' | '=' | '?' | '@' | '[' | ']' | '~' => {
                    format!("%{:02X}", ch as u8)
                }
                _ => ch.to_string(),
            })
            .collect::<String>();

        format!("{}:{}", name, encoded_value)
    } else {
        input.to_string()
    }
}

async fn gpt4_body_handle(req: &HttpRequest, body: &mut Option<Json<Value>>) {
    if req.uri().path().contains("/backend-api/conversation") && req.method().as_str() == "POST" {
        if let Some(body) = body.as_mut().and_then(|b| b.as_object_mut()) {
            if let Some(v) = body.get("model").and_then(|m| m.as_str()) {
                if body.get("arkose_token").is_none() {
                    if let Ok(arkose) = ArkoseToken::new_from_endpoint(v).await {
                        let _ = body.insert("arkose_token".to_owned(), json!(arkose));
                    }
                }
            }
        }
    }
}

async fn check_self_ip(client: &Client) {
    match client
        .get("https://ifconfig.me")
        .timeout(Duration::from_secs(10))
        .header(header::ACCEPT, "application/json")
        .send()
        .await
    {
        Ok(resp) => match resp.text().await {
            Ok(res) => {
                info!("What is my IP address: {}", res.trim())
            }
            Err(err) => {
                warn!("Check IP address error: {}", err.to_string())
            }
        },
        Err(err) => {
            warn!("Check IP request error: {}", err)
        }
    }
}
