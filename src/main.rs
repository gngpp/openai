use anyhow::Context;
use clap::{Parser, Subcommand};
use std::{io::Write, path::PathBuf, sync::Once};

#[derive(Parser, Debug)]
#[clap(author, version, about, arg_required_else_help = true)]
struct Opt {
    /// Enable debug
    #[clap(long, global = true, env = "OPENGPT_DEBUG", value_parser = initialize_log)]
    debug: bool,

    /// HTTP Proxy. Format: protocol://user:pass@ip:port
    #[clap(short, long, env = "OPENGPT_PROXY", value_parser = parse_proxy_url)]
    proxy: Option<reqwest::Proxy>,

    /// OpenAI gpt-3.5-turbo chat api, Note: OpenAI will bill you
    #[clap(short, long, env = "OPENGPT_TURBO")]
    turbo: bool,

    /// OpenAI account email, Format: gngppz@gmail.com
    #[arg(short = 'E', long, env = "OPENGPT_EMAIL", requires = "password")]
    email: Option<String>,

    /// OpenAI account password
    #[arg(short = 'W', long, env = "OPENGPT_PASSWORD", requires = "email")]
    password: Option<String>,

    #[clap(subcommand)]
    command: Option<SubCommands>,
}

#[derive(Subcommand, Debug)]
enum SubCommands {
    /// Start proxy server
    Server {
        /// Server Listen host
        #[clap(short = 'H', long, default_value = "0.0.0.0", value_parser = parse_host)]
        host: std::net::IpAddr,
        /// Server Listen port
        #[clap(short = 'P', long, default_value = "7999", value_parser = parse_port_in_range)]
        port: u16,
    },
    /// Setting configuration
    Config {
        /// Working directory, refresh_token will be stored in there if specified
        #[clap(short = 'W', long)]
        workdir: Option<PathBuf>,
    },
}
use std::collections::HashMap;

use fficall::model::Identifier;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _opt = Opt::parse();
    let email = std::env::var("EMAIL")?;
    let password = std::env::var("PASSWORD")?;
    let store = openai::token::FileStore::default();
    let mut auth = openai::oauth::OAuthBuilder::builder()
        .email(email)
        .password(password)
        .cache(true)
        .cookie_store(true)
        .token_store(store)
        .client_timeout(std::time::Duration::from_secs(20))
        .build();
    let token = auth.do_get_access_token().await?;
    let mut headers = HashMap::new();
    headers.insert(reqwest::header::USER_AGENT.to_string(),
     "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36".to_string());
    headers.insert(
        reqwest::header::AUTHORIZATION.to_string(),
        token.get_bearer_access_token().to_owned(),
    );

    let payload = json!(
        {
            "action": "next",
            "messages": [
              {
                "id": "ec526640-1cac-4a8d-a4c4-5102ccbcacbc",
                "author": {
                  "role": "user"
                },
                "content": {
                  "content_type": "text",
                  "parts": [
                    "Rust Examples"
                  ]
                }
              }
            ],
            "parent_message_id": "e8a1841c-2694-4434-ad4d-ed0b79813879",
            "model": "text-davinci-002-render-sha",
            "timezone_offset_min": -480,
            "history_and_training_disabled": false
          }
    );
    let payload = fficall::model::RequestPayloadBuilder::default()
        .request_url("https://chat.openai.com/backend-api/conversation".to_string())
        .request_method(fficall::model::RequestMethod::POST)
        .tls_client_identifier(Identifier::Chrome105)
        .headers(headers)
        .request_body(payload.to_string())
        .timeout_seconds(200 as u32)
        .without_cookie_jar(false)
        .build()
        .unwrap();
    fficall::call_request_stream(payload)?;
    // let body = gohttp::call_request(payload)?;
    // if body.is_success() {
    //     println!("{:#?}", body);
    // } else {
    //     println!("{:?}", body)
    // }

    Ok(())
}

static INIT: Once = Once::new();

fn initialize_log(s: &str) -> anyhow::Result<bool> {
    let debug = s.parse::<bool>()?;
    match debug {
        true => std::env::set_var("RUST_LOG", "DEBUG"),
        false => std::env::set_var("RUST_LOG", "INFO"),
    };
    INIT.call_once(|| {
        env_logger::builder()
            .format(|buf, record| {
                writeln!(
                    buf,
                    "{} {}: {}",
                    record.level(),
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    record.args()
                )
            })
            .init();
    });
    Ok(debug)
}

const PORT_RANGE: std::ops::RangeInclusive<usize> = 1024..=65535;

// port range parse
fn parse_port_in_range(s: &str) -> anyhow::Result<u16> {
    let port: usize = s
        .parse()
        .map_err(|_| anyhow::anyhow!(format!("`{}` isn't a port number", s)))?;
    if PORT_RANGE.contains(&port) {
        return Ok(port as u16);
    }
    anyhow::bail!(format!(
        "Port not in range {}-{}",
        PORT_RANGE.start(),
        PORT_RANGE.end()
    ))
}

// address parse
fn parse_host(s: &str) -> anyhow::Result<std::net::IpAddr> {
    let addr = s
        .parse::<std::net::IpAddr>()
        .map_err(|_| anyhow::anyhow!(format!("`{}` isn't a ip address", s)))?;
    Ok(addr)
}

fn parse_proxy_url(proxy_url: &str) -> anyhow::Result<reqwest::Proxy> {
    let url = url::Url::parse(proxy_url)
        .context("The Proxy Url format must be `protocol://user:pass@ip:port`")?;
    let protocol = url.scheme().to_string();
    match protocol.as_str() {
        "http" => "80".to_string(),
        "https" => "443".to_string(),
        _ => anyhow::bail!("Unsupported protocol: {}", protocol),
    };

    Ok(reqwest::Proxy::all(url.to_string())?)
}
