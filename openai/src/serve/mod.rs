pub mod middleware;
#[cfg(feature = "sign")]
pub mod sign;
#[cfg(feature = "limit")]
pub mod tokenbucket;

use actix_web::http::header;
use actix_web::middleware::Logger;
use actix_web::web::Json;
use actix_web::{get, patch, post, App, HttpResponse, HttpServer, Responder};
use actix_web::{web, HttpRequest};
use derive_builder::Builder;
use reqwest::Client;
use serde_json::Value;
use std::fs::File;
use std::io::BufReader;
use std::sync::Once;
use std::time::Duration;

use std::net::IpAddr;
use std::path::PathBuf;
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};

use crate::oauth::OAuthClient;
use crate::serve::tokenbucket::TokenBucket;
use crate::{info, oauth};

use super::api::{HEADER_UA, URL_API, URL_CHATGPT_BACKEND, URL_CHATGPT_PUBLIC};

static INIT: Once = Once::new();
static mut CLIENT: Option<Client> = None;
static mut OAUTH_CLIENT: Option<OAuthClient> = None;

fn client() -> Client {
    if let Some(client) = unsafe { &CLIENT } {
        return client.clone();
    }
    panic!("The requesting client must be initialized")
}

fn oauth_client() -> OAuthClient {
    if let Some(oauth_client) = unsafe { &OAUTH_CLIENT } {
        return oauth_client.clone();
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
    /// TCP keepalive (second)
    tcp_keepalive: Duration,
    /// TLS keypair
    tls_keypair: Option<(PathBuf, PathBuf)>,
    /// Enable url signature (signature secret key)
    #[cfg(feature = "sign")]
    sign_secret_key: Option<String>,
    /// Enable Tokenbucket
    tb_enable: bool,
    /// Tokenbucket capacity
    #[cfg(feature = "limit")]
    tb_capacity: u32,
    /// Tokenbucket fill rate
    #[cfg(feature = "limit")]
    tb_fill_rate: u32,
    /// Tokenbucket expired (second)
    #[cfg(feature = "limit")]
    tb_expired: u32,
}

impl Launcher {
    pub async fn run(self) -> anyhow::Result<()> {
        use reqwest::header;
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(HEADER_UA),
        );
        let client = reqwest::ClientBuilder::new()
            .chrome_builder(reqwest::browser::ChromeVersion::V105)
            .default_headers(headers)
            .tcp_keepalive(Some(self.tcp_keepalive))
            .pool_max_idle_per_host(self.workers)
            .cookie_store(false)
            .build()?;

        let oauth_client = oauth::OAuthClientBuilder::builder()
            .cookie_store(true)
            .pool_max_idle_per_host(self.workers)
            .build();

        unsafe {
            INIT.call_once(|| {
                CLIENT = Some(client);
                OAUTH_CLIENT = Some(oauth_client);
            });
        }

        info!(
            "Starting HTTP(S) server at http(s)://{}:{}",
            self.host, self.port
        );

        let serve = HttpServer::new(move || {
            let app = App::new()
                .wrap(Logger::default())
                .service(web::scope("/api"))
                .service(
                    web::scope("/backend-api")
                        .wrap(middleware::TokenAuthorization)
                        .service(get_models)
                        .service(get_account_check)
                        .service(get_account_check_v4)
                        .service(get_settings_beta_features)
                        .service(get_conversation)
                        .service(get_conversations)
                        .service(post_conversation)
                        .service(post_conversation_gen_title)
                        .service(post_conversation_message_feedback)
                        .service(patch_conversation)
                        .service(patch_conversations),
                )
                .service(web::scope("/public-api").service(get_conversation_limit))
                .service(
                    web::scope("/oauth")
                        .service(post_access_token)
                        .service(post_refresh_token)
                        .service(post_revoke_token),
                )
                .service(
                    web::scope("/dashboard")
                        .service(get_api_key_list)
                        .service(post_api_key)
                        .service(post_dashboard_login)
                        .service(get_billing_usage),
                );

            #[cfg(all(not(feature = "sign"), feature = "limit"))]
            {
                return app.wrap(middleware::TokenBucketRateLimiter::new(TokenBucket::new(
                    self.tb_enable,
                    self.tb_capacity,
                    self.tb_fill_rate,
                    self.tb_expired,
                )));
            }

            #[cfg(all(not(feature = "limit"), feature = "sign"))]
            {
                return app.wrap(middleware::ApiSign::new(self.sign_secret_key.clone()));
            }

            #[cfg(all(feature = "sign", feature = "limit"))]
            {
                return app
                    .wrap(middleware::ApiSign::new(self.sign_secret_key.clone()))
                    .wrap(middleware::TokenBucketRateLimiter::new(TokenBucket::new(
                        self.tb_enable,
                        self.tb_capacity,
                        self.tb_fill_rate,
                        self.tb_expired,
                    )));
            }

            #[cfg(not(any(feature = "sign", feature = "limit")))]
            app
        })
        .keep_alive(self.tcp_keepalive)
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
            None => anyhow::bail!("Could not locate PKCS 8 private keys."),
        }
    }
}

#[post("/token")]
async fn post_access_token(account: Json<oauth::OAuthAccount>) -> impl Responder {
    match oauth_client().do_access_token(account.into_inner()).await {
        Ok(token) => HttpResponse::Ok().json(token),
        Err(err) => response_oauth_bad_handle(&err.to_string()),
    }
}

#[post("/refresh_token")]
async fn post_refresh_token(req: HttpRequest) -> impl Responder {
    if let Some(token) = req.headers().get(header::AUTHORIZATION) {
        match token.to_str() {
            Ok(token_val) => {
                let token_val = token_val.trim_start_matches("Bearer ");
                match oauth_client().do_refresh_token(token_val).await {
                    Ok(token) => HttpResponse::Ok().json(token),
                    Err(err) => response_oauth_bad_handle(&err.to_string()),
                }
            }
            Err(err) => HttpResponse::InternalServerError().json(err.to_string()),
        }
    } else {
        HttpResponse::Unauthorized().body(r#"{ "message": "refresh_token is required! "}"#)
    }
}

#[post("/revoke_token")]
async fn post_revoke_token(req: HttpRequest) -> impl Responder {
    if let Some(token) = req.headers().get(header::AUTHORIZATION) {
        match token.to_str() {
            Ok(token_val) => {
                let token_val = token_val.trim_start_matches("Bearer ");
                match oauth_client().do_revoke_token(token_val).await {
                    Ok(_) => HttpResponse::Ok().finish(),
                    Err(err) => response_oauth_bad_handle(&err.to_string()),
                }
            }
            Err(err) => HttpResponse::InternalServerError().json(err.to_string()),
        }
    } else {
        HttpResponse::Unauthorized().body(r#"{ "message": "refresh_token is required! "}"#)
    }
}

#[post("/onboarding/login")]
async fn post_dashboard_login(req: HttpRequest) -> impl Responder {
    let resp = client()
        .post(format!("{URL_API}/dashboard/onboarding/login"))
        .headers(header_convert(req.headers()))
        .send()
        .await;
    response_handle(resp)
}

#[post("/user/api_keys")]
async fn post_api_key(req: HttpRequest, body: Json<Value>) -> impl Responder {
    let resp = client()
        .post(format!("{URL_API}/dashboard/user/api_keys"))
        .headers(header_convert(req.headers()))
        .json(&body.0)
        .send()
        .await;
    response_handle(resp)
}

#[get("/user/api_keys")]
async fn get_api_key_list(req: HttpRequest) -> impl Responder {
    let resp = client()
        .get(format!("{URL_API}/dashboard/user/api_keys"))
        .headers(header_convert(req.headers()))
        .send()
        .await;
    response_handle(resp)
}

///  https://api.openai.com/dashboard/billing/usage?end_date=2022-11-01&start_date=2022-10-01
#[get("billing/usage")]
async fn get_billing_usage(req: HttpRequest) -> impl Responder {
    let resp = client()
        .get(format!(
            "{URL_API}/dashboard/billing/usage?{}",
            req.query_string()
        ))
        .headers(header_convert(req.headers()))
        .send()
        .await;
    response_handle(resp)
}

#[get("/models")]
async fn get_models(req: HttpRequest) -> impl Responder {
    let resp = client()
        .get(format!(
            "{URL_CHATGPT_BACKEND}/models?{}",
            req.query_string()
        ))
        .headers(header_convert(req.headers()))
        .send()
        .await;
    response_handle(resp)
}

#[get("/accounts/check")]
async fn get_account_check(req: HttpRequest) -> impl Responder {
    let resp = client()
        .get(format!("{URL_CHATGPT_BACKEND}/accounts/check"))
        .headers(header_convert(req.headers()))
        .send()
        .await;
    response_handle(resp)
}

#[get("/accounts/check/v4-2023-04-27")]
async fn get_account_check_v4(req: HttpRequest) -> impl Responder {
    let resp = client()
        .get(format!(
            "{URL_CHATGPT_BACKEND}/accounts/check/v4-2023-04-27"
        ))
        .headers(header_convert(req.headers()))
        .send()
        .await;
    response_handle(resp)
}

/// chatgpt plus
#[get("/settings/beta_features")]
async fn get_settings_beta_features(req: HttpRequest) -> impl Responder {
    let resp = client()
        .get(format!("{URL_CHATGPT_BACKEND}/settings/beta_features"))
        .headers(header_convert(req.headers()))
        .send()
        .await;
    response_handle(resp)
}

#[get("/conversation/{conversation_id}")]
async fn get_conversation(req: HttpRequest, conversation_id: web::Path<String>) -> impl Responder {
    let resp = client()
        .get(format!(
            "{URL_CHATGPT_BACKEND}/conversation/{}",
            conversation_id.into_inner()
        ))
        .headers(header_convert(req.headers()))
        .send()
        .await;
    response_handle(resp)
}

#[get("/conversations")]
async fn get_conversations<'a>(req: HttpRequest) -> impl Responder {
    let resp = client()
        .get(format!(
            "{URL_CHATGPT_BACKEND}/conversations?{}",
            req.query_string()
        ))
        .headers(header_convert(req.headers()))
        .send()
        .await;
    response_handle(resp)
}

#[post("/conversation")]
async fn post_conversation(req: HttpRequest, mut body: Json<Value>) -> impl Responder {
    if let Some(body) = body.0.as_object_mut() {
        use crate::api::models::{ArkoseToken, GPT4Model};
        let model = body.get("model");
        if let Some(v) = model {
            if let Some(str) = v.as_str() {
                if GPT4Model::try_from(str).is_ok() {
                    match serde_json::to_value(ArkoseToken) {
                        Ok(x) => {
                            let _ = body.insert("arkose_token".to_owned(), x);
                        }
                        Err(err) => {
                            return HttpResponse::InternalServerError().json(err.to_string());
                        }
                    }
                }
            }
        }
    }
    let resp = client()
        .post(format!("{URL_CHATGPT_BACKEND}/conversation"))
        .headers(header_convert(req.headers()))
        .json(&body.0)
        .send()
        .await;
    response_handle(resp)
}

#[patch("/conversation/{conversation_id}")]
async fn patch_conversation(
    req: HttpRequest,
    conversation_id: web::Path<String>,
    body: Json<Value>,
) -> impl Responder {
    let resp = client()
        .patch(format!(
            "{URL_CHATGPT_BACKEND}/conversation/{conversation_id}"
        ))
        .headers(header_convert(req.headers()))
        .json(&body.0)
        .send()
        .await;
    response_handle(resp)
}

#[patch("/conversations")]
async fn patch_conversations(req: HttpRequest, body: Json<Value>) -> impl Responder {
    let resp = client()
        .patch(format!("{URL_CHATGPT_BACKEND}/conversations"))
        .headers(header_convert(req.headers()))
        .json(&body.0)
        .send()
        .await;
    response_handle(resp)
}

#[post("/conversation/gen_title/{conversation_id}")]
async fn post_conversation_gen_title(
    req: HttpRequest,
    conversation_id: web::Path<String>,
    body: Json<Value>,
) -> impl Responder {
    let resp = client()
        .post(format!(
            "{URL_CHATGPT_BACKEND}/conversation/gen_title/{conversation_id}"
        ))
        .headers(header_convert(req.headers()))
        .json(&body.0)
        .send()
        .await;
    response_handle(resp)
}

#[post("/conversation/message_feedbak")]
async fn post_conversation_message_feedback(req: HttpRequest, body: Json<Value>) -> impl Responder {
    let resp = client()
        .post(format!(
            "{URL_CHATGPT_BACKEND}/conversation/message_feedbak"
        ))
        .headers(header_convert(req.headers()))
        .json(&body.0)
        .send()
        .await;
    response_handle(resp)
}

#[get("/conversation_limit")]
async fn get_conversation_limit(req: HttpRequest) -> impl Responder {
    let resp = client()
        .get(format!("{URL_CHATGPT_PUBLIC}/conversation_limit"))
        .headers(header_convert(req.headers()))
        .send()
        .await;
    response_handle(resp)
}

fn header_convert(headers: &actix_web::http::header::HeaderMap) -> reqwest::header::HeaderMap {
    headers
        .iter()
        .filter(|v| {
            let h = v.0;
            h.ne(&header::CONNECTION) && h.ne(&header::USER_AGENT)
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn response_oauth_bad_handle(msg: &str) -> HttpResponse {
    HttpResponse::BadRequest().json(msg)
}

fn response_handle(resp: Result<reqwest::Response, reqwest::Error>) -> HttpResponse {
    match resp {
        Ok(resp) => {
            let status = resp.status();
            let mut builder = HttpResponse::build(status);
            resp.headers().into_iter().for_each(|kv| {
                builder.insert_header(kv);
            });
            builder.streaming(resp.bytes_stream())
        }
        Err(err) => HttpResponse::InternalServerError().json(err.to_string()),
    }
}
