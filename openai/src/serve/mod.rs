pub mod middleware;
#[cfg(feature = "sign")]
pub mod sign;
#[cfg(feature = "limit")]
pub mod tokenbucket;

use actix_web::http::header;
use actix_web::middleware::Logger;
use actix_web::web::Json;
use actix_web::{post, App, HttpResponse, HttpServer, Responder};
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
use crate::{error, info, oauth};

use super::api::{HEADER_UA, URL_CHATGPT_API, URL_PLATFORM_API};

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
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .chrome_builder(reqwest::browser::ChromeVersion::V105)
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
                .service(
                    web::scope("/oauth")
                        .service(post_access_token)
                        .service(post_refresh_token)
                        .service(post_revoke_token),
                )
                .service(
                    web::resource("/dashboard/{tail:.*}")
                        .wrap(middleware::TokenAuthorization)
                        .route(web::to(official_proxy)),
                )
                .service(
                    web::resource("/v1/{tail:.*}")
                        .wrap(middleware::TokenAuthorization)
                        .route(web::to(official_proxy)),
                )
                .service(
                    web::resource("/backend-api/{tail:.*}")
                        .wrap(middleware::TokenAuthorization)
                        .route(web::to(unofficial_proxy)),
                )
                .service(web::resource("/public-api/{tail:.*}").route(web::to(unofficial_proxy)));

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
    match req.headers().get(header::AUTHORIZATION) {
        Some(token) => match token.to_str() {
            Ok(token_val) => {
                let refresh_token = token_val.trim_start_matches("Bearer ");
                match oauth_client().do_refresh_token(refresh_token).await {
                    Ok(token) => HttpResponse::Ok().json(token),
                    Err(err) => response_oauth_bad_handle(&err.to_string()),
                }
            }
            Err(err) => HttpResponse::InternalServerError().json(err.to_string()),
        },
        None => HttpResponse::Unauthorized().body(r#"{ "message": "refresh_token is required! "}"#),
    }
}

#[post("/revoke_token")]
async fn post_revoke_token(req: HttpRequest) -> impl Responder {
    match req.headers().get(header::AUTHORIZATION) {
        Some(token) => match token.to_str() {
            Ok(token_val) => {
                let refresh_token = token_val.trim_start_matches("Bearer ");
                match oauth_client().do_revoke_token(refresh_token).await {
                    Ok(_) => HttpResponse::Ok().finish(),
                    Err(err) => response_oauth_bad_handle(&err.to_string()),
                }
            }
            Err(err) => HttpResponse::InternalServerError().json(err.to_string()),
        },
        None => HttpResponse::Unauthorized().body(r#"{ "message": "refresh_token is required! "}"#),
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
    let builder = client()
        .request(
            req.method().clone(),
            format!("{URL_PLATFORM_API}{}", req.uri()),
        )
        .headers(header_convert(req.headers()));
    let resp = match body {
        Some(body) => builder.json(&body).send().await,
        None => builder.send().await,
    };
    response_handle(resp)
}

async fn unofficial_proxy(req: HttpRequest, mut body: Option<Json<Value>>) -> impl Responder {
    gpt4_body_handle(&req, &mut body);
    let builder = client()
        .request(
            req.method().clone(),
            format!("{URL_CHATGPT_API}{}", req.uri()),
        )
        .headers(header_convert(req.headers()));
    let resp = match body {
        Some(body) => builder.json(&body).send().await,
        None => builder.send().await,
    };
    response_handle(resp)
}

fn header_convert(headers: &actix_web::http::header::HeaderMap) -> reqwest::header::HeaderMap {
    headers
        .iter()
        .filter(|v| v.0.eq(&header::AUTHORIZATION))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn gpt4_body_handle(req: &HttpRequest, body: &mut Option<Json<Value>>) {
    if req.uri().path().contains("/backend-api/conversation") && req.method().as_str() == "POST" {
        if let Some(body) = body.as_mut().and_then(|b| b.as_object_mut()) {
            use crate::api::models::{ArkoseToken, GPT4Model};
            if let Some(v) = body.get("model").and_then(|m| m.as_str()) {
                if GPT4Model::try_from(v).is_ok() {
                    if let Ok(x) = serde_json::to_value(ArkoseToken) {
                        let _ = body.insert("arkose_token".to_owned(), x);
                    }
                }
            }
        }
    }
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
