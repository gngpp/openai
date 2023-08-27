use crate::util;
use clap::{Args, Parser, Subcommand};
use openai::serve::tokenbucket;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
        /// Configuration template output to file (toml format file)
        #[clap(short, long, env = "OPENGPT_SERVE_GT_OUT", group = "gt")]
        out: Option<PathBuf>,
        /// Edit configuration template file
        #[clap(short, long, group = "gt")]
        edit: Option<PathBuf>,
    },
}

#[derive(Args, Debug, Default, Serialize, Deserialize)]
pub(super) struct ServeArgs {
    /// Configuration file path (toml format file)
    #[clap(short = 'C', long, env = "OPENGPT_CONFIG", value_parser = util::parse_config)]
    pub(super) config: Option<PathBuf>,
    /// Server Listen host
    #[clap(short = 'H', long, env = "OPENGPT_HOST", default_value = "0.0.0.0", value_parser = util::parse_host)]
    pub(super) host: Option<std::net::IpAddr>,
    /// Server Listen port
    #[clap(short = 'P', long, env = "OPENGPT_PORT", default_value = "7999", value_parser = util::parse_port_in_range)]
    pub(super) port: Option<u16>,
    /// Server worker-pool size (Recommended number of CPU cores)
    #[clap(short = 'W', long, env = "OPENGPT_WORKERS", default_value = "1")]
    pub(super) workers: usize,
    /// Concurrent limit (Enforces a limit on the concurrent number of requests the underlying)
    #[clap(long, env = "OPENGPT_CONCUURENT_LIMIT", default_value = "65535")]
    pub(super) concurrent_limit: usize,
    /// Server proxies pool, Example: protocol://user:pass@ip:port
    #[clap(long, env = "OPENGPT_PROXY", value_parser = util::parse_proxies_url)]
    pub(super) proxies: Option<std::vec::Vec<String>>,
    /// Client timeout (seconds)
    #[clap(long, env = "OPENGPT_TIMEOUT", default_value = "600")]
    pub(super) timeout: usize,
    /// Client connect timeout (seconds)
    #[clap(long, env = "OPENGPT_CONNECT_TIMEOUT", default_value = "60")]
    pub(super) connect_timeout: usize,
    /// TCP keepalive (seconds)
    #[clap(long, env = "OPENGPT_TCP_KEEPALIVE", default_value = "60")]
    pub(super) tcp_keepalive: usize,
    /// TLS certificate file path
    #[clap(long, env = "OPENGPT_TLS_CERT", requires = "tls_key")]
    pub(super) tls_cert: Option<PathBuf>,
    /// TLS private key file path (EC/PKCS8/RSA)
    #[clap(long, env = "OPENGPT_TLS_KEY", requires = "tls_cert")]
    pub(super) tls_key: Option<PathBuf>,
    /// PUID cookie value of Plus account
    #[clap(long, env = "OPENGPT_PUID")]
    pub(super) puid: Option<String>,
    /// Obtain the PUID of the Plus account user. Supports split: ':', '-', '--' ... , Example: `user:pass` or `user:pass:mfa`
    #[clap(long, env = "OPENGPT_PUID_USER", value_parser = util::parse_puid_user)]
    pub(super) puid_user: Option<(String, String, Option<String>)>,
    /// Web UI api prefix
    #[clap(long, env = "OPENGPT_UI_API_PREFIX", value_parser = util::parse_url)]
    pub(super) api_prefix: Option<String>,
    /// Arkose endpoint, Example: https://client-api.arkoselabs.com
    #[clap(long, env = "OPENGPT_ARKOSE_ENDPOINT", value_parser = util::parse_url)]
    pub(super) arkose_endpoint: Option<String>,
    /// Get arkose-token endpoint
    #[clap(short = 'A', long, env = "OPENGPT_ARKOSE_TOKEN_ENDPOINT", value_parser = util::parse_url)]
    pub(super) arkose_token_endpoint: Option<String>,
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
    /// Token bucket redis url(support cluster), Example: redis://user:pass@ip:port
    #[clap(
        long,
        env = "OPENGPT_TB_REDIS_URL",
        default_value = "redis://127.0.0.1:6379",
        requires = "tb_enable",
        value_parser = util::parse_proxies_url
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
    /// Token bucket expired (seconds)
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
    /// Terminal interaction
    Terminal,
}
