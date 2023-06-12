use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use derive_builder::Builder;
use openssl::ssl::SslAcceptor;
use openssl::ssl::SslFiletype;
use openssl::ssl::SslMethod;

use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::path::PathBuf;

#[derive(Builder)]
pub struct Config {
    #[builder(default = "IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))")]
    host: IpAddr,
    #[builder(default = "7999")]
    port: u16,
    #[builder(default = "1")]
    workers: usize,
    tls_cert: Option<PathBuf>,
    tls_key: Option<PathBuf>,
}

pub async fn run(config: Config) -> std::io::Result<()> {
    let serve = HttpServer::new(|| {
        App::new()
            .service(hello)
            .service(echo)
            .route("/hey", web::get().to(manual_hello))
    })
    .keep_alive(None)
    .workers(config.workers);
    match config.tls_key {
        Some(key) => {
            let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
            builder.set_private_key_file(key, SslFiletype::PEM).unwrap();
            builder.set_certificate_chain_file(config.tls_cert.unwrap())?;
            serve
                .bind_openssl((config.host, config.port), builder)?
                .run()
                .await
        }
        None => serve.bind((config.host, config.port))?.run().await,
    }
}

#[get("/")]
async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

#[post("/echo")]
async fn echo(req_body: String) -> impl Responder {
    HttpResponse::Ok().body(req_body)
}

async fn manual_hello() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}
