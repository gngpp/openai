pub mod middleware;
#[cfg(feature = "sign")]
pub mod sign;
#[cfg(feature = "limit")]
pub mod tokenbucket;

#[cfg(feature = "template")]
pub mod template;

use actix_web::http::header;
use actix_web::middleware::Logger;
use actix_web::web::Json;
use actix_web::{get, post, App, HttpResponse, HttpServer, Responder};
use actix_web::{web, HttpRequest};

use derive_builder::Builder;
use reqwest::browser::ChromeVersion;
use reqwest::Client;
use serde_json::Value;
use std::fs::File;
use std::io::BufReader;
use std::sync::Once;
use std::time::Duration;
use url::Url;

use std::net::IpAddr;
use std::path::PathBuf;
use tokio_rustls::rustls::{Certificate, PrivateKey, ServerConfig};

use crate::arkose::ArkoseToken;
use crate::auth::OAuthClient;
use crate::serve::template::TemplateData;
use crate::serve::tokenbucket::TokenBucketContext;
use crate::{auth, info};

use crate::{HEADER_UA, URL_CHATGPT_API, URL_PLATFORM_API};
const EMPTY: &str = "";
static INIT: Once = Once::new();
static mut CLIENT: Option<Client> = None;
static mut OAUTH_CLIENT: Option<OAuthClient> = None;

pub(super) fn client() -> Client {
    if let Some(client) = unsafe { &CLIENT } {
        return client.clone();
    }
    panic!("The requesting client must be initialized")
}

pub(super) fn oauth_client() -> OAuthClient {
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
    tcp_keepalive: usize,
    /// Client timeout
    timeout: usize,
    /// Client connect timeout
    connect_timeout: usize,
    /// TLS keypair
    tls_keypair: Option<(PathBuf, PathBuf)>,
    /// Web UI api prefix
    api_prefix: Option<Url>,
    /// Enable url signature (signature secret key)
    #[cfg(feature = "sign")]
    sign_secret_key: Option<String>,
    /// Enable Tokenbucket
    #[cfg(feature = "limit")]
    tb_enable: bool,
    /// Tokenbucket store strategy
    #[cfg(feature = "limit")]
    tb_store_strategy: tokenbucket::Strategy,
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
        let client = reqwest::Client::builder()
            .user_agent(HEADER_UA)
            .chrome_builder(ChromeVersion::V108)
            .tcp_keepalive(Some(Duration::from_secs((self.tcp_keepalive + 1) as u64)))
            .timeout(Duration::from_secs((self.timeout + 1) as u64))
            .connect_timeout(Duration::from_secs((self.connect_timeout + 1) as u64))
            .pool_max_idle_per_host(self.workers)
            .cookie_store(false)
            .build()?;

        let oauth_client = auth::OAuthClientBuilder::builder()
            .user_agent(HEADER_UA)
            .chrome_builder(ChromeVersion::V108)
            .cookie_store(true)
            .timeout(Duration::from_secs((self.timeout + 1) as u64))
            .connect_timeout(Duration::from_secs((self.connect_timeout + 1) as u64))
            .pool_max_idle_per_host(self.workers)
            .build();

        INIT.call_once(|| unsafe {
            CLIENT = Some(client);
            OAUTH_CLIENT = Some(oauth_client);
        });

        info!(
            "Starting HTTP(S) server at http(s)://{}:{}",
            self.host, self.port
        );

        if let Some(url) = &self.api_prefix {
            info!("Web UI site use api: {url}")
        }

        // template data
        let template_data = TemplateData {
            api_prefix: self.api_prefix.map_or("".to_owned(), |v| v.to_string()),
        };
        // serve
        let serve = HttpServer::new(move || {
            let app = App::new()
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
                // imgae(picture) endpoint
                .service(web::resource("/_next/image").route(web::to(unofficial_proxy)))
                // auth endpoint
                .service(post_access_token)
                .service(post_refresh_token)
                .service(post_revoke_token)
                .service(get_arkose_token)
                // static files endpoint
                // templates data
                .app_data(template_data.clone())
                // templates page endpoint
                .configure(template::config);

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
async fn post_access_token(account: web::Form<auth::OAuthAccount>) -> impl Responder {
    match oauth_client().do_access_token(&account.into_inner()).await {
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
    match oauth_client().do_refresh_token(refresh_token).await {
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
    match oauth_client().do_revoke_token(refresh_token).await {
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
    let builder = client()
        .request(
            req.method().clone(),
            format!("{URL_PLATFORM_API}{}?{}", req.path(), req.query_string()),
        )
        .headers(header_convert(req.headers()));
    let resp = match body {
        Some(body) => builder.json(&body).send().await,
        None => builder.send().await,
    };
    response_handle(resp)
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
    let builder = client()
        .request(
            req.method().clone(),
            format!("{URL_CHATGPT_API}{}?{}", req.path(), req.query_string()),
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

async fn gpt4_body_handle(req: &HttpRequest, body: &mut Option<Json<Value>>) {
    if req.uri().path().contains("/backend-api/conversation") && req.method().as_str() == "POST" {
        if let Some(body) = body.as_mut().and_then(|b| b.as_object_mut()) {
            if let Some(v) = body.get("model").and_then(|m| m.as_str()) {
                match ArkoseToken::new(v).await {
                    Ok(arkose) => {
                        if let Ok(x) = serde_json::to_value(arkose) {
                            let _ = body.insert("arkose_token".to_owned(), x);
                        }
                    }
                    Err(_) => {}
                }
            }
        }
    }
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
