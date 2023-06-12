use anyhow::Context;
use clap::{Parser, Subcommand};
use openai::serve::ConfigBuilder;
use std::{io::Write, path::PathBuf, sync::Once};

pub mod account;
pub mod prompt;
pub mod util;

#[derive(Parser)]
#[clap(author, version, about)]
struct Opt {
    /// Enable debug
    #[clap(long, global = true, env = "OPENGPT_DEBUG", value_parser = initialize_log)]
    debug: bool,

    #[clap(subcommand)]
    command: Option<SubCommands>,
}

#[derive(Subcommand)]
enum SubCommands {
    /// Start the http server
    Serve {
        /// Server Listen host
        #[clap(short = 'H', long, env = "OPENGPT_HOST", default_value = "0.0.0.0", value_parser = parse_host)]
        host: Option<std::net::IpAddr>,
        /// Server Listen port
        #[clap(short = 'P', long, env = "OPENGPT_PORT", default_value = "7999", value_parser = parse_port_in_range)]
        port: Option<u16>,
        /// Server worker-pool size (Recommended number of CPU cores)
        #[clap(short, long, env = "OPENGPT_WORKERS", default_value = "1")]
        workers: Option<usize>,
        /// TLS certificate file path
        #[clap(long, env = "OPENGPT_TLS_CERT", requires = "tls_key")]
        tls_cert: Option<PathBuf>,
        /// TLS private key file path
        #[clap(long, env = "OPENGPT_TLS_KEY", requires = "tls_cert")]
        tls_key: Option<PathBuf>,
    },
    /// Account configuration settings
    Account,
    /// Configuration Settings
    Config {
        /// Working directory, refresh_token will be stored in there if specified
        #[clap(short, long, env = "OPENGPT_WORKDIR")]
        workdir: Option<PathBuf>,

        /// Unofficial API prefix. Format: https://example.com/backend-api
        #[clap(long, env = "OPENGPT_API")]
        unofficial_api: Option<String>,

        /// Unofficial API http proxy. Format: protocol://user:pass@ip:port
        #[clap(long, env = "OPENGPT_PROXY", value_parser = parse_proxy_url)]
        unofficial_proxy: Option<url::Url>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _opt = Opt::parse();
    match _opt.command {
        Some(command) => match command {
            SubCommands::Account => {
                prompt::account_prompt()?;
            }
            SubCommands::Config {
                workdir,
                unofficial_api,
                unofficial_proxy,
            } => {

            },
            SubCommands::Serve {
                host,
                port,
                workers,
                tls_cert,
                tls_key,
            } => {
                let conf = ConfigBuilder::default()
                    .host(host.unwrap())
                    .port(port.unwrap())
                    .workers(workers.unwrap())
                    .tls_cert(tls_cert)
                    .tls_key(tls_key)
                    .build()?;
                openai::serve::run(conf).await?
            }
        },
        None => prompt::main_prompt()?,
    }
    Ok(())
}

pub(crate) static ONCE_INIT: Once = Once::new();

fn initialize_log(s: &str) -> anyhow::Result<bool> {
    let debug = s.parse::<bool>()?;
    match debug {
        true => std::env::set_var("RUST_LOG", "DEBUG"),
        false => std::env::set_var("RUST_LOG", "INFO"),
    };
    ONCE_INIT.call_once(|| {
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

// proxy proto
fn parse_proxy_url(proxy_url: &str) -> anyhow::Result<url::Url> {
    let url = url::Url::parse(proxy_url)
        .context("The Proxy Url format must be `protocol://user:pass@ip:port`")?;
    let protocol = url.scheme().to_string();
    match protocol.as_str() {
        "http" | "https" | "sockt5" => Ok(url),
        _ => anyhow::bail!("Unsupported protocol: {}", protocol),
    }
}
