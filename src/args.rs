use crate::parse;
use clap::{Args, Subcommand};
use openai::{arkose::funcaptcha::solver::Solver, proxy};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(all(feature = "serve", not(feature = "terminal")))]
pub mod cmd {
    use super::ServeSubcommand;
    use clap::Parser;

    #[derive(Parser)]
    #[clap(author, version, about, arg_required_else_help = true)]
    pub struct Opt {
        #[clap(subcommand)]
        pub command: Option<ServeSubcommand>,
    }
}

#[cfg(all(feature = "serve", feature = "terminal"))]
pub mod cmd {
    use super::{ServeSubcommand, Subcommand};
    use clap::Parser;
    #[derive(Parser)]
    #[clap(author, version, about, arg_required_else_help = true)]
    pub struct Opt {
        #[clap(subcommand)]
        pub command: Option<SubCommands>,
    }

    #[allow(clippy::large_enum_variant)]
    #[derive(Subcommand)]
    pub enum SubCommands {
        /// Start the http server
        #[clap(subcommand)]
        Serve(ServeSubcommand),
        /// Terminal interaction
        Terminal,
    }
}

#[derive(Subcommand)]
pub enum ServeSubcommand {
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
    /// Generate MITM CA certificate
    Genca,
    /// Show the impersonate user-agent list
    UA,
    /// Generate config template file (toml format file)
    GT {
        /// Configuration template output to file (toml format file)
        #[clap(short, long, group = "gt")]
        out: Option<PathBuf>,
    },
    /// Update the application
    Update,
}

#[derive(Args, Debug, Default, Serialize, Deserialize)]
pub struct ServeArgs {
    /// Log level (info/debug/warn/trace/error)
    #[clap(short = 'L', long, global = true, env = "LOG", default_value = "info")]
    pub(super) level: String,

    /// Configuration file path (toml format file)
    #[clap(short = 'C', long, env = "CONFIG", value_parser = parse::parse_file_path)]
    pub(super) config: Option<PathBuf>,

    /// Server bind address
    #[clap(short, long, env = "BIND", default_value = "0.0.0.0:7999", value_parser = parse::parse_socket_addr)]
    pub(super) bind: Option<std::net::SocketAddr>,

    /// Server Enforces a limit on the concurrent number of requests the underlying
    #[clap(long, default_value = "1024")]
    pub(super) concurrent_limit: usize,

    /// Server/Client timeout (seconds)
    #[clap(long, default_value = "360")]
    pub(super) timeout: usize,

    /// Server/Client connect timeout (seconds)
    #[clap(long, default_value = "5")]
    pub(super) connect_timeout: usize,

    /// Server/Client TCP keepalive (seconds)
    #[clap(long, default_value = "60")]
    pub(super) tcp_keepalive: usize,

    /// No TCP keepalive (Client)
    #[clap(short = 'H', long, env = "NO_TCP_KEEPALIVE", default_value = "false")]
    pub(super) no_keepalive: bool,

    /// Keep the client alive on an idle socket with an optional timeout set
    #[clap(long, default_value = "90")]
    pub(super) pool_idle_timeout: usize,

    /// Client proxy, support multiple proxy, use ',' to separate, Format: proto|type
    /// Proto: all/api/auth/arkose, default: all
    /// Type: interface/proxy/ipv6 subnet，proxy type only support: socks5/http/https
    /// Example: all|socks5://192.168.1.1:1080, api|10.0.0.1, auth|2001:db8::/32, http://192.168.1.1:1081
    #[clap(short = 'x',long, env = "PROXIES", value_parser = parse::parse_proxies_url, verbatim_doc_comment)]
    pub(super) proxies: Option<std::vec::Vec<proxy::Proxy>>,

    /// Enable direct connection
    #[clap(long, env = "ENABLE_DIRECT")]
    pub(super) enable_direct: bool,

    /// Impersonate User-Agent, separate multiple ones with ","
    #[clap(short = 'I',long, env = "IMPERSONATE_UA", value_parser = parse::parse_impersonate_uas, verbatim_doc_comment)]
    pub(super) impersonate_uas: Option<std::vec::Vec<String>>,

    /// Enabled Cookie Store
    #[clap(long, env = "COOKIE_STORE")]
    pub(super) cookie_store: bool,

    /// Use fastest DNS resolver
    #[clap(long, env = "FASTEST_DNS")]
    pub(super) fastest_dns: bool,

    /// TLS certificate file path
    #[clap(long, env = "TLS_CERT", requires = "tls_key")]
    pub(super) tls_cert: Option<PathBuf>,

    /// TLS private key file path (EC/PKCS8/RSA)
    #[clap(long, env = "TLS_KEY", requires = "tls_cert")]
    pub(super) tls_key: Option<PathBuf>,

    /// Cloudflare turnstile captcha site key
    #[clap(long, env = "CF_SECRET_KEY", requires = "cf_secret_key")]
    pub(super) cf_site_key: Option<String>,

    /// Cloudflare turnstile captcha secret key
    #[clap(long, env = "CF_SITE_KEY", requires = "cf_site_key")]
    pub(super) cf_secret_key: Option<String>,

    /// Login Authentication Key
    #[clap(short = 'A', long, env = "AUTH_KEY")]
    pub(super) auth_key: Option<String>,

    /// Disable WebUI
    #[clap(short = 'D', long, env = "DISABLE_WEBUI")]
    pub(super) disable_webui: bool,

    /// Enable file endpoint proxy
    #[clap(short = 'F', long, env = "ENABLE_FILE_PROXY")]
    pub(super) enable_file_proxy: bool,

    /// Enable arkose token endpoint proxy
    #[clap(short = 'G', long, env = "ENABLE_ARKOSE_PROXY")]
    pub(super) enable_arkose_proxy: bool,

    /// Visitor email whitelist
    #[clap(short = 'W', long, env = "VISITOR_EMAIL_WHITELIST", value_parser = parse::parse_email_whitelist)]
    pub(super) visitor_email_whitelist: Option<std::vec::Vec<String>>,

    /// Arkose endpoint, Example: https://client-api.arkoselabs.com
    #[clap(long, value_parser = parse::parse_url)]
    pub(super) arkose_endpoint: Option<String>,

    /// Enable Arkose GPT-3.5 experiment
    #[clap(short = 'E', long, default_value = "false")]
    pub(super) arkose_gpt3_experiment: bool,

    /// Enable Arkose GPT-3.5 experiment solver
    #[clap(short = 'S', long, default_value = "false")]
    pub(super) arkose_gpt3_experiment_solver: bool,

    /// About the browser HAR directory path requested by ChatGPT GPT-3.5 ArkoseLabs
    #[clap(long, value_parser = parse::parse_dir_path)]
    pub(super) arkose_gpt3_har_dir: Option<PathBuf>,

    /// About the browser HAR directory path requested by ChatGPT GPT-4 ArkoseLabs
    #[clap(long, value_parser = parse::parse_dir_path)]
    pub(super) arkose_gpt4_har_dir: Option<PathBuf>,

    ///  About the browser HAR directory path requested by Auth ArkoseLabs
    #[clap(long, value_parser = parse::parse_dir_path)]
    pub(super) arkose_auth_har_dir: Option<PathBuf>,

    /// About the browser HAR directory path requested by Platform ArkoseLabs
    #[clap(long, value_parser = parse::parse_dir_path)]
    pub(super) arkose_platform_har_dir: Option<PathBuf>,

    /// HAR file upload authenticate key
    #[clap(short = 'K', long)]
    pub(super) arkose_har_upload_key: Option<String>,

    /// About ArkoseLabs solver platform
    #[clap(
        short = 's',
        long,
        default_value = "fcsrv",
        requires = "arkose_solver_key"
    )]
    pub(super) arkose_solver: Solver,

    /// About the solver client key by ArkoseLabs
    #[clap(short = 'k', long)]
    pub(super) arkose_solver_key: Option<String>,

    /// About the solver client endpoint by ArkoseLabs
    #[clap(long, value_parser = parse::parse_url, requires = "arkose_solver_key")]
    pub(super) arkose_solver_endpoint: Option<String>,

    /// About the solver submit multiple image limit by ArkoseLabs
    #[clap(long, default_value = "1", requires = "arkose_solver_key")]
    pub(super) arkose_solver_limit: usize,

    /// About the solver tguess endpoint by ArkoseLabs
    #[clap(long, value_parser = parse::parse_url)]
    pub(super) arkose_solver_tguess_endpoint: Option<String>,

    /// About the solver image store directory by ArkoseLabs
    #[clap(long, value_parser = parse::parse_dir_path)]
    pub(super) arkose_solver_image_dir: Option<PathBuf>,

    /// Enable token bucket flow limitation
    #[clap(short = 'T', long)]
    #[cfg(feature = "limit")]
    pub(super) tb_enable: bool,

    /// Token bucket store strategy (mem/redb)
    #[clap(long, default_value = "mem", requires = "tb_enable")]
    #[cfg(feature = "limit")]
    pub(super) tb_strategy: String,

    /// Token bucket capacity
    #[clap(long, default_value = "60", requires = "tb_enable")]
    #[cfg(feature = "limit")]
    pub(super) tb_capacity: u32,

    /// Token bucket fill rate
    #[clap(long, default_value = "1", requires = "tb_enable")]
    #[cfg(feature = "limit")]
    pub(super) tb_fill_rate: u32,

    /// Token bucket expired (seconds)
    #[clap(long, default_value = "86400", requires = "tb_enable")]
    #[cfg(feature = "limit")]
    pub(super) tb_expired: u32,

    /// Preauth MITM server bind address
    #[clap(
    short = 'B',
    long,
    env = "PREAUTH_BIND",
    value_parser = parse::parse_socket_addr,
    )]
    pub(super) pbind: Option<std::net::SocketAddr>,

    /// Preauth MITM server upstream proxy
    /// Supports: http/https/socks5/socks5h
    #[clap(
    short = 'X',
    long,
    env = "PREAUTH_UPSTREAM",
    value_parser = parse::parse_url,
    requires = "pbind",
    verbatim_doc_comment
    )]
    pub(super) pupstream: Option<String>,

    /// Preauth MITM server CA certificate file path
    #[clap(long, default_value = "ca/cert.crt", requires = "pbind")]
    pub(super) pcert: PathBuf,

    /// Preauth MITM server CA private key file path
    #[clap(long, default_value = "ca/key.pem", requires = "pbind")]
    pub(super) pkey: PathBuf,
}
