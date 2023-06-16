use anyhow::Context;
use clap::{Parser, Subcommand};
use openai::serve::LauncherBuilder;
use std::{io::Write, path::PathBuf, sync::Once, time::Duration};

pub mod account;
pub mod prompt;
pub mod ui;
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
        #[clap(short = 'W', long, env = "OPENGPT_WORKERS", default_value = "1")]
        workers: usize,
        /// TCP keepalive (second)
        #[clap(long, env = "OPENGPT_TCP_KEEPALIVE", default_value = "5")]
        tcp_keepalive: usize,
        /// TLS certificate file path
        #[clap(long, env = "OPENGPT_TLS_CERT", requires = "tls_key")]
        tls_cert: Option<PathBuf>,
        /// TLS private key file path (EC)
        #[clap(long, env = "OPENGPT_TLS_KEY", requires = "tls_cert")]
        tls_key: Option<PathBuf>,
        /// Enable token bucket flow limitation
        #[clap(short = 'T', long, env = "OPENGPT_TB_ENABLE")]
        tb_enable: bool,
        /// Token bucket capacity
        #[clap(
            long,
            env = "OPENGPT_TB_CAPACITY",
            default_value = "60",
            requires = "tb_enable"
        )]
        tb_capacity: u32,
        /// Token bucket fill rate
        #[clap(
            long,
            env = "OPENGPT_TB_FILL_RATE",
            default_value = "1",
            requires = "tb_enable"
        )]
        tb_fill_rate: u32,
        /// Token bucket expired (second)
        #[clap(
            long,
            env = "OPENGPT_TB_EXPIRED",
            default_value = "86400",
            requires = "tb_enable"
        )]
        tb_expired: u32,
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
                workdir: _,
                unofficial_api: _,
                unofficial_proxy: _,
            } => {}
            SubCommands::Serve {
                host,
                port,
                workers,
                tcp_keepalive,
                tls_cert,
                tls_key,
                tb_enable,
                tb_capacity,
                tb_fill_rate,
                tb_expired,
            } => {
                let mut builder = LauncherBuilder::default();
                let mut builder = builder
                    .host(host.unwrap())
                    .port(port.unwrap())
                    .tls_keypair(None)
                    .tcp_keepalive(Duration::from_secs(tcp_keepalive as u64))
                    .workers(workers)
                    .tb_enable(tb_enable)
                    .tb_capacity(tb_capacity)
                    .tb_fill_rate(tb_fill_rate)
                    .tb_expired(tb_expired);

                if tls_key.is_some() && tls_cert.is_some() {
                    builder = builder.tls_keypair(Some((tls_cert.unwrap(), tls_key.unwrap())));
                }
                builder.build()?.run().await?
            }
        },
        None => prompt::main_prompt()?,
    }
    Ok(())
}

static ONCE_INIT: Once = Once::new();

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
