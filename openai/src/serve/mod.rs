pub mod middleware;
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
use std::sync::{Arc, Once};

use rustls::{Certificate, PrivateKey, ServerConfig};
use std::net::IpAddr;
use std::path::PathBuf;

use crate::info;
use crate::serve::tokenbucket::TokenBucket;

use super::api::{HEADER_UA, URL_CHATGPT_BASE};

static INIT: Once = Once::new();
static mut CLIENT: Option<Arc<Client>> = None;

fn initialize_client(client: Client) {
    unsafe {
        INIT.call_once(|| {
            CLIENT = Some(Arc::new(client));
        });
    }
}

fn client() -> &'static Client {
    if let Some(client) = unsafe { &CLIENT } {
        return client;
    }
    panic!("The requesting client must be initialized")
}

#[derive(Builder, Clone)]
pub struct Launcher {
    host: IpAddr,
    port: u16,
    workers: usize,
    tls_keypair: Option<(PathBuf, PathBuf)>,
    /// Enable token bucket flow limitation
    tb_enable: bool,
    /// Token bucket capacity
    tb_capacity: u32,
    /// Token bucket fill rate
    tb_fill_rate: u32,
    /// Token bucket expired (second)
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
            .cookie_store(false)
            .build()
            .unwrap();

        initialize_client(client);

        info!(
            "Starting HTTP(S) server at http(s)://{}:{}",
            self.host, self.port
        );
        let ctls_keypaird = self.tls_keypair.clone();
        let default_serve = async {
            let serve = HttpServer::new(move || {
                App::new()
                    .wrap(Logger::default())
                    .wrap(middleware::TokenAuthorization)
                    .service(get_models)
                    .service(get_account_check)
                    .service(get_conversation)
                    .service(get_conversations)
                    .service(post_conversation)
                    .service(post_conversation_gen_title)
                    .service(post_conversation_message_feedback)
                    .service(patch_conversation)
                    .service(patch_conversations)
            })
            .keep_alive(None)
            .workers(self.workers);
            match ctls_keypaird {
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
        };

        #[cfg(feature = "limit")]
        if self.tb_enable {
            let serve = HttpServer::new(move || {
                let mw = TokenBucket::new(self.tb_capacity, self.tb_fill_rate, self.tb_expired);
                App::new()
                    .wrap(Logger::default())
                    .wrap(middleware::TokenBucketRateLimiter::new(mw))
                    .wrap(middleware::TokenAuthorization)
                    .service(get_models)
                    .service(get_account_check)
                    .service(get_conversation)
                    .service(get_conversations)
                    .service(post_conversation)
                    .service(post_conversation_gen_title)
                    .service(post_conversation_message_feedback)
                    .service(patch_conversation)
                    .service(patch_conversations)
            })
            .keep_alive(None)
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
        } else {
            default_serve.await
        }
        #[cfg(not(feature = "limit"))]
        default_serve.await
    }

    async fn load_rustls_config(
        tls_cert: PathBuf,
        tls_key: PathBuf,
    ) -> anyhow::Result<ServerConfig> {
        use rustls_pemfile::{certs, pkcs8_private_keys};

        // init server config builder with safe defaults
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth();

        // load TLS key/cert files
        let cert_file = &mut BufReader::new(File::open(tls_cert)?);
        let key_file = &mut BufReader::new(File::open(tls_key)?);

        // convert files to key/cert objects
        let cert_chain = certs(cert_file)?.into_iter().map(Certificate).collect();
        let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_file)?
            .into_iter()
            .map(PrivateKey)
            .collect();

        // exit if no keys could be parsed
        if keys.is_empty() {
            anyhow::bail!("Could not locate PKCS 8 private keys.")
        }

        Ok(config.with_single_cert(cert_chain, keys.remove(0))?)
    }
}

#[get("/backend-api/models")]
async fn get_models(req: HttpRequest) -> impl Responder {
    match client()
        .get(format!("{URL_CHATGPT_BASE}/models"))
        .headers(header_convert(req.headers()))
        .send()
        .await
    {
        Ok(resp) => response_handle(resp),
        Err(err) => response_bad_handle(err),
    }
}

#[post("/backend-api/accounts/check")]
async fn get_account_check(req: HttpRequest) -> impl Responder {
    match client()
        .get(format!("{URL_CHATGPT_BASE}/accounts/check"))
        .headers(header_convert(req.headers()))
        .send()
        .await
    {
        Ok(resp) => response_handle(resp),
        Err(err) => response_bad_handle(err),
    }
}

#[get("/backend-api/conversation/{conversation_id}")]
async fn get_conversation(req: HttpRequest, conversation_id: web::Path<String>) -> impl Responder {
    match client()
        .get(format!(
            "{URL_CHATGPT_BASE}/conversation/{}",
            conversation_id.into_inner()
        ))
        .headers(header_convert(req.headers()))
        .send()
        .await
    {
        Ok(resp) => response_handle(resp),
        Err(err) => response_bad_handle(err),
    }
}

#[get("/backend-api/conversations")]
async fn get_conversations(
    req: HttpRequest,
    param: web::Query<ConversationsQuery>,
) -> impl Responder {
    let param = param.into_inner();
    match client()
        .get(format!(
            "{URL_CHATGPT_BASE}/conversations?offset={}&limit={}&order={}",
            param.offset, param.limit, param.order
        ))
        .headers(header_convert(req.headers()))
        .send()
        .await
    {
        Ok(resp) => response_handle(resp),
        Err(err) => response_bad_handle(err),
    }
}

#[post("/backend-api/conversation")]
async fn post_conversation(req: HttpRequest, body: Json<Value>) -> impl Responder {
    match client()
        .post(format!("{URL_CHATGPT_BASE}/conversation"))
        .headers(header_convert(req.headers()))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => response_handle(resp),
        Err(err) => response_bad_handle(err),
    }
}

#[patch("/backend-api/conversation/{conversation_id}")]
async fn patch_conversation(
    req: HttpRequest,
    conversation_id: web::Path<String>,
    body: Json<Value>,
) -> impl Responder {
    match client()
        .patch(format!("{URL_CHATGPT_BASE}/conversation/{conversation_id}"))
        .headers(header_convert(req.headers()))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => response_handle(resp),
        Err(err) => response_bad_handle(err),
    }
}

#[patch("/backend-api/conversations")]
async fn patch_conversations(req: HttpRequest, body: Json<Value>) -> impl Responder {
    match client()
        .patch(format!("{URL_CHATGPT_BASE}/conversations"))
        .headers(header_convert(req.headers()))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => response_handle(resp),
        Err(err) => response_bad_handle(err),
    }
}

#[post("/backend-api/conversation/gen_title/{conversation_id}")]
async fn post_conversation_gen_title(
    req: HttpRequest,
    conversation_id: web::Path<String>,
    body: Json<Value>,
) -> impl Responder {
    match client()
        .post(format!(
            "{URL_CHATGPT_BASE}/conversation/gen_title/{conversation_id}"
        ))
        .headers(header_convert(req.headers()))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => response_handle(resp),
        Err(err) => response_bad_handle(err),
    }
}

#[post("/backend-api/conversation/message_feedbak")]
async fn post_conversation_message_feedback(req: HttpRequest, body: Json<Value>) -> impl Responder {
    match client()
        .post(format!("{URL_CHATGPT_BASE}/conversation/message_feedbak"))
        .headers(header_convert(req.headers()))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => response_handle(resp),
        Err(err) => response_bad_handle(err),
    }
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

fn response_bad_handle(err: reqwest::Error) -> HttpResponse {
    HttpResponse::BadGateway()
        .insert_header((header::CONTENT_TYPE, "application/json"))
        .body(err.to_string())
}

fn response_handle(resp: reqwest::Response) -> HttpResponse {
    let status = resp.status();
    let mut builder = HttpResponse::build(status);
    resp.headers().into_iter().for_each(|kv| {
        builder.insert_header(kv);
    });
    builder.streaming(resp.bytes_stream())
}

use serde::Deserialize;

#[derive(Deserialize)]
struct ConversationsQuery {
    #[serde(default = "ConversationsQuery::default_offset")]
    offset: u32,
    #[serde(default = "ConversationsQuery::default_limit")]
    limit: u32,
    #[serde(default = "ConversationsQuery::default_order")]
    order: String,
}

impl ConversationsQuery {
    fn default_offset() -> u32 {
        0
    }
    fn default_limit() -> u32 {
        20
    }

    fn default_order() -> String {
        String::from("updated")
    }
}
