use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use openai::serve::tokenbucket;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, str::FromStr};

#[derive(Parser)]
#[clap(author, version, about)]
pub(super) struct Opt {
    #[clap(subcommand)]
    pub(super) command: Option<SubCommands>,
    /// Log level (info/debug/warn/trace/error)
    #[clap(
        short = 'L',
        long,
        global = true,
        env = "OPENGPT_LOG_LEVEL",
        default_value = "info"
    )]
    pub(super) level: String,
}

#[derive(Subcommand)]
pub(super) enum ServeSubcommand {
    /// Run the HTTP server
    Run(ServeArgs),
    /// Stop the HTTP server daemon
    #[cfg(target_family = "unix")]
    Stop,
    /// Start the HTTP server daemon
    #[cfg(target_family = "unix")]
    Start(ServeArgs),
    /// Restart the HTTP server daemon
    #[cfg(target_family = "unix")]
    Restart(ServeArgs),
    /// Status of the Http server daemon process
    #[cfg(target_family = "unix")]
    Status,
    /// Show the Http server daemon log
    #[cfg(target_family = "unix")]
    Log,
    /// Generate config template file (toml format file)
    GT {
        /// Overwrite existing configuration file
        #[clap(short, long, env = "OPENGPT_SERVE_GT_COVER")]
        cover: bool,
        /// Configuration template output to file (toml format file)
        #[clap(short, long, env = "OPENGPT_SERVE_GT_OUT")]
        out: Option<PathBuf>,
    },
}

#[derive(Args, Debug, Default, Serialize, Deserialize)]
pub(super) struct ServeArgs {
    /// Configuration file path (toml format file)
    #[clap(short = 'C', long, env = "OPENGPT_CONFIG", value_parser = parse_config)]
    pub(super) config: Option<PathBuf>,
    /// Server Listen host
    #[clap(short = 'H', long, env = "OPENGPT_HOST", default_value = "0.0.0.0", value_parser = parse_host)]
    pub(super) host: Option<std::net::IpAddr>,
    /// Server Listen port
    #[clap(short = 'P', long, env = "OPENGPT_PORT", default_value = "7999", value_parser = parse_port_in_range)]
    pub(super) port: Option<u16>,
    /// Server worker-pool size (Recommended number of CPU cores)
    #[clap(short = 'W', long, env = "OPENGPT_WORKERS", default_value = "1")]
    pub(super) workers: usize,
    /// Server proxies pool, example: protocol://user:pass@ip:port
    #[clap(long, env = "OPENGPT_PROXY", value_parser = parse_url)]
    pub(super) proxies: Option<std::vec::Vec<String>>,
    /// Client timeout (secends)
    #[clap(long, env = "OPENGPT_TIMEOUT", default_value = "600")]
    pub(super) timeout: usize,
    /// Client connect timeout (secends)
    #[clap(long, env = "OPENGPT_CONNECT_TIMEOUT", default_value = "60")]
    pub(super) connect_timeout: usize,
    /// TCP keepalive (secends)
    #[clap(long, env = "OPENGPT_TCP_KEEPALIVE", default_value = "60")]
    pub(super) tcp_keepalive: usize,
    /// TLS certificate file path
    #[clap(long, env = "OPENGPT_TLS_CERT", requires = "tls_key")]
    pub(super) tls_cert: Option<PathBuf>,
    /// TLS private key file path (EC/PKCS8/RSA)
    #[clap(long, env = "OPENGPT_TLS_KEY", requires = "tls_cert")]
    pub(super) tls_key: Option<PathBuf>,
    /// Web UI api prefix
    #[clap(long, env = "OPENGPT_UI_API_PREFIX", value_parser = parse_url)]
    pub(super) api_prefix: Option<String>,
    /// Enable url signature (signature secret key)
    #[clap(short = 'S', long, env = "OPENGPT_SIGNATURE")]
    #[cfg(feature = "sign")]
    pub(super) sign_secret_key: Option<String>,
    /// Enable token bucket flow limitation
    #[clap(short = 'T', long, env = "OPENGPT_TB_ENABLE")]
    #[cfg(feature = "limit")]
    pub(super) tb_enable: bool,
    /// Token bucket store strategy (mem/redis)
    #[clap(
        long,
        env = "OPENGPT_TB_STORE_STRATEGY",
        default_value = "mem",
        requires = "tb_enable"
    )]
    #[cfg(feature = "limit")]
    pub(super) tb_store_strategy: tokenbucket::Strategy,
    /// Token bucket redis url(support cluster), example: redis://user:pass@ip:port
    #[clap(
        long,
        env = "OPENGPT_TB_REDIS_URL",
        default_value = "redis://127.0.0.1:6379",
        requires = "tb_enable",
        value_parser = parse_url
    )]
    #[cfg(feature = "limit")]
    pub(super) tb_redis_url: std::vec::Vec<String>,
    /// Token bucket capacity
    #[clap(
        long,
        env = "OPENGPT_TB_CAPACITY",
        default_value = "60",
        requires = "tb_enable"
    )]
    #[cfg(feature = "limit")]
    pub(super) tb_capacity: u32,
    /// Token bucket fill rate
    #[clap(
        long,
        env = "OPENGPT_TB_FILL_RATE",
        default_value = "1",
        requires = "tb_enable"
    )]
    #[cfg(feature = "limit")]
    pub(super) tb_fill_rate: u32,
    /// Token bucket expired (second)
    #[clap(
        long,
        env = "OPENGPT_TB_EXPIRED",
        default_value = "86400",
        requires = "tb_enable"
    )]
    #[cfg(feature = "limit")]
    pub(super) tb_expired: u32,

    /// Cloudflare turnstile captcha site key
    #[clap(long, env = "OPENGPT_CF_SITE_KEY", requires = "cf_secret_key")]
    pub(super) cf_site_key: Option<String>,
    /// Cloudflare turnstile captcha secret key
    #[clap(long, env = "OPENGPT_CF_SECRET_KEY", requires = "cf_site_key")]
    pub(super) cf_secret_key: Option<String>,

    /// Disable WebUI
    #[clap(short = 'D', long, env = "OPENGPT_DISABLE_WEBUI")]
    pub(super) disable_webui: bool,
}

#[derive(Subcommand)]
pub(super) enum SubCommands {
    /// Start the http server
    #[clap(subcommand)]
    Serve(ServeSubcommand),
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
        #[clap(long, env = "OPENGPT_PROXY", value_parser = parse_url)]
        unofficial_proxy: Option<url::Url>,
    },
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
fn parse_url(s: &str) -> anyhow::Result<Vec<String>> {
    let split = s.split(",");
    let mut proxies: Vec<_> = vec![];
    for ele in split {
        let url = url::Url::parse(ele)
            .context("The Proxy Url format must be `protocol://user:pass@ip:port`")?;
        let protocol = url.scheme().to_string();
        match protocol.as_str() {
            "http" | "https" | "socks5" | "redis" => proxies.push(ele.to_string()),
            _ => anyhow::bail!("Unsupported protocol: {}", protocol),
        };
    }
    Ok(proxies)
}

fn parse_config(s: &str) -> anyhow::Result<PathBuf> {
    let path =
        PathBuf::from_str(s).map_err(|_| anyhow::anyhow!(format!("`{}` isn't a path", s)))?;
    match path.exists() {
        true => Ok(path),
        false => {
            if let Some(parent) = path.parent() {
                parent.exists().then(|| ()).ok_or_else(|| {
                    anyhow::anyhow!(format!("Path {} not exists", parent.display()))
                })?;
            }
            Ok(path)
        }
    }
}
