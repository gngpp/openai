use anyhow::Context;
use clap::{Parser, Subcommand};
use openai::serve::{tokenbucket, LauncherBuilder};
use std::{
    io::Write,
    path::PathBuf,
    sync::{Arc, Once},
};
use url::Url;

pub mod account;
pub mod prompt;
pub mod ui;
pub mod util;

#[derive(Parser)]
#[clap(author, version, about)]
struct Opt {
    #[clap(subcommand)]
    command: Option<SubCommands>,
}

#[derive(Subcommand)]
enum SubCommands {
    /// Start the http server
    Serve {
        /// Enable debug
        #[clap(short = 'D', long, env = "OPENGPT_DEBUG", value_parser = initialize_log)]
        debug: bool,
        /// Server Listen host
        #[clap(short = 'H', long, env = "OPENGPT_HOST", default_value = "0.0.0.0", value_parser = parse_host)]
        host: Option<std::net::IpAddr>,
        /// Server Listen port
        #[clap(short = 'P', long, env = "OPENGPT_PORT", default_value = "7999", value_parser = parse_port_in_range)]
        port: Option<u16>,
        /// Server worker-pool size (Recommended number of CPU cores)
        #[clap(short = 'W', long, env = "OPENGPT_WORKERS", default_value = "1")]
        workers: usize,
        /// Client timeout(secends)
        #[clap(long, env = "OPENGPT_TIMEOUT", default_value = "600")]
        timeout: usize,
        /// Client connect timeout(secends)
        #[clap(long, env = "OPENGPT_CONNECT_TIMEOUT", default_value = "10")]
        connect_timeout: usize,
        /// TCP keepalive (secends)
        #[clap(long, env = "OPENGPT_TCP_KEEPALIVE", default_value = "5")]
        tcp_keepalive: usize,
        /// TLS certificate file path
        #[clap(long, env = "OPENGPT_TLS_CERT", requires = "tls_key")]
        tls_cert: Option<PathBuf>,
        /// TLS private key file path (EC/PKCS8/RSA)
        #[clap(long, env = "OPENGPT_TLS_KEY", requires = "tls_cert")]
        tls_key: Option<PathBuf>,
        /// Web UI api prefix
        #[clap(long, env = "OPENGPT_UI_API_PREFIX", value_parser = parse_proxy_url)]
        api_prefix: Option<Url>,
        /// Enable url signature (signature secret key)
        #[clap(short = 'S', long, env = "OPENGPT_SIGNATURE")]
        #[cfg(feature = "sign")]
        sign_secret_key: Option<String>,
        /// Enable token bucket flow limitation
        #[clap(short = 'T', long, env = "OPENGPT_TB_ENABLE")]
        #[cfg(feature = "limit")]
        tb_enable: bool,
        /// Token bucket store strategy (mem/redis)
        #[clap(
            long,
            env = "OPENGPT_TB_STORE_STRATEGY",
            default_value = "mem",
            requires = "tb_enable"
        )]
        #[cfg(feature = "limit")]
        tb_store_strategy: tokenbucket::Strategy,
        /// Token bucket capacity
        #[clap(
            long,
            env = "OPENGPT_TB_CAPACITY",
            default_value = "60",
            requires = "tb_enable"
        )]
        #[cfg(feature = "limit")]
        tb_capacity: u32,
        /// Token bucket fill rate
        #[clap(
            long,
            env = "OPENGPT_TB_FILL_RATE",
            default_value = "1",
            requires = "tb_enable"
        )]
        #[cfg(feature = "limit")]
        tb_fill_rate: u32,
        /// Token bucket expired (second)
        #[clap(
            long,
            env = "OPENGPT_TB_EXPIRED",
            default_value = "86400",
            requires = "tb_enable"
        )]
        #[cfg(feature = "limit")]
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
                debug: _,
                host,
                port,
                workers,
                timeout,
                connect_timeout,
                tcp_keepalive,
                tls_cert,
                tls_key,
                api_prefix,
                #[cfg(feature = "sign")]
                sign_secret_key,
                #[cfg(feature = "limit")]
                tb_enable,
                #[cfg(feature = "limit")]
                tb_store_strategy,
                #[cfg(feature = "limit")]
                tb_capacity,
                #[cfg(feature = "limit")]
                tb_fill_rate,
                #[cfg(feature = "limit")]
                tb_expired,
            } => {
                let mut builder = LauncherBuilder::default();
                let builder = builder
                    .host(host.unwrap())
                    .port(port.unwrap())
                    .tls_keypair(None)
                    .tcp_keepalive(tcp_keepalive)
                    .timeout(timeout)
                    .connect_timeout(connect_timeout)
                    .api_prefix(api_prefix)
                    .workers(workers);

                #[cfg(feature = "limit")]
                let builder = builder
                    .tb_enable(tb_enable)
                    .tb_store_strategy(tb_store_strategy)
                    .tb_capacity(tb_capacity)
                    .tb_fill_rate(tb_fill_rate)
                    .tb_expired(tb_expired);

                #[cfg(feature = "limit")]
                let mut builder = builder.sign_secret_key(sign_secret_key);

                if tls_key.is_some() && tls_cert.is_some() {
                    builder = builder.tls_keypair(Some((tls_cert.unwrap(), tls_key.unwrap())));
                }
                builder.build()?.run().await?
            }
        },
        None => {
            let (sync_io_tx, mut sync_io_rx) = tokio::sync::mpsc::channel::<ui::io::IoEvent>(100);

            // We need to share the App between thread
            let app = Arc::new(tokio::sync::Mutex::new(ui::app::App::new(
                sync_io_tx.clone(),
            )));
            let app_ui = Arc::clone(&app);

            // Configure log
            tui_logger::init_logger(log::LevelFilter::Debug)?;
            tui_logger::set_default_level(log::LevelFilter::Debug);

            // Handle IO in a specifc thread
            tokio::spawn(async move {
                let mut handler = ui::io::handler::IoAsyncHandler::new(app);
                while let Some(io_event) = sync_io_rx.recv().await {
                    handler.handle_io_event(io_event).await;
                }
            });

            ui::start_ui(&app_ui).await?
        }
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
        "http" | "https" => Ok(url),
        _ => anyhow::bail!("Unsupported protocol: {}", protocol),
    }
}
