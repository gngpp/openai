use crate::{arkose::funcaptcha::solver::ArkoseSolver, proxy};
use reqwest::impersonate::Impersonate;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};
use typed_builder::TypedBuilder;

#[derive(TypedBuilder, Clone, Default)]
pub struct Args {
    /// Server bind address
    #[builder(setter(into), default = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 7999)))]
    pub(crate) bind: Option<SocketAddr>,

    /// Server concurrent limit (Enforces a limit on the concurrent number of requests the underlying)
    #[builder(setter(into), default = 65535)]
    pub(crate) concurrent_limit: usize,

    /// Enabled Cookie Store
    #[builder(default = false)]
    pub(crate) cookie_store: bool,

    /// Use fastest DNS resolver
    #[builder(default = false)]
    pub(crate) fastest_dns: bool,

    /// Server/Client TCP keepalive (second)
    #[builder(setter(into), default = 75)]
    pub(crate) tcp_keepalive: usize,

    /// Disable Http Server/Client Keepalive
    #[builder(default = false)]
    pub(crate) no_keepalive: bool,

    /// Keep the client alive on an idle socket with an optional timeout set
    #[builder(setter(into), default = 90)]
    pub(crate) pool_idle_timeout: usize,

    /// Server/Client timeout
    #[builder(setter(into), default = 600)]
    pub(crate) timeout: usize,

    /// Server/Client connect timeout
    #[builder(setter(into), default = 60)]
    pub(crate) connect_timeout: usize,

    /// Disable direct connection
    #[builder(default = false)]
    pub(crate) enable_direct: bool,

    /// Client proxies
    #[builder(setter(into), default)]
    pub(crate) proxies: Vec<proxy::Proxy>,

    /// Random User-Agent
    #[builder(setter(into), default = Some(vec![Impersonate::OkHttp4_9]))]
    pub(crate) impersonate_uas: Option<Vec<Impersonate>>,

    /// TLS cert
    #[builder(setter(into), default)]
    pub(crate) tls_cert: Option<PathBuf>,

    /// TLS key
    #[builder(setter(into), default)]
    pub(crate) tls_key: Option<PathBuf>,

    /// Visitor email whitelist
    #[builder(setter(into), default)]
    pub(super) visitor_email_whitelist: Option<Vec<String>>,

    /// Login auth key
    #[builder(setter(into), default)]
    pub(super) auth_key: Option<String>,

    /// Disable web ui
    #[builder(setter(into), default = false)]
    pub(crate) disable_ui: bool,

    /// Enable file proxy
    #[builder(setter(into), default = false)]
    pub(crate) enable_file_proxy: bool,

    /// Get arkose token proxy
    #[builder(default = false)]
    pub(crate) enable_arkose_proxy: bool,

    /// Cloudflare captcha site key
    #[builder(setter(into), default)]
    pub(crate) cf_site_key: Option<String>,

    /// Cloudflare captcha secret key
    #[builder(setter(into), default)]
    pub(crate) cf_secret_key: Option<String>,

    /// Arkose endpoint
    #[builder(setter(into), default)]
    pub(crate) arkose_endpoint: Option<String>,

    /// ChatGPT GPT-3.5 Arkoselabs HAR record file path
    #[builder(setter(into), default)]
    pub(crate) arkose_gpt3_har_dir: Option<PathBuf>,

    /// ChatGPT GPT-4 Arkoselabs HAR record file path
    #[builder(setter(into), default)]
    pub(crate) arkose_gpt4_har_dir: Option<PathBuf>,

    /// Auth Arkoselabs HAR record file path
    #[builder(setter(into), default)]
    pub(crate) arkose_auth_har_dir: Option<PathBuf>,

    /// Platform Arkoselabs HAR record file path
    #[builder(setter(into), default)]
    pub(crate) arkose_platform_har_dir: Option<PathBuf>,

    /// Enable Arkose GPT-3.5 experiment
    #[builder(setter(into), default = false)]
    pub(crate) arkose_gpt3_experiment: bool,

    /// Enable Arkose GPT-3.5 experiment solver
    #[builder(setter(into), default = false)]
    pub(crate) arkose_gpt3_experiment_solver: bool,

    /// HAR file upload authenticate key
    #[builder(setter(into), default)]
    pub(crate) arkose_har_upload_key: Option<String>,

    /// arkoselabs solver
    #[builder(setter(into), default)]
    pub(crate) arkose_solver: Option<ArkoseSolver>,

    /// About the solver tguess endpoint by ArkoseLabs
    #[builder(setter(into), default)]
    pub(crate) arkose_solver_tguess_endpoint: Option<String>,

    /// About the solver image store directory by ArkoseLabs
    #[builder(setter(into), default)]
    pub(crate) arkose_solver_image_dir: Option<PathBuf>,

    /// Enable Tokenbucket
    #[cfg(feature = "limit")]
    #[builder(setter(into), default = false)]
    pub(crate) tb_enable: bool,

    /// Tokenbucket store strategy
    #[cfg(feature = "limit")]
    #[builder(setter(into), default = "mem".to_string())]
    pub(crate) tb_strategy: String,

    /// Tokenbucket capacity
    #[cfg(feature = "limit")]
    #[builder(setter(into), default = 60)]
    pub(crate) tb_capacity: u32,

    /// Tokenbucket fill rate
    #[cfg(feature = "limit")]
    #[builder(setter(into), default = 1)]
    pub(crate) tb_fill_rate: u32,

    /// Tokenbucket expired (second)
    #[cfg(feature = "limit")]
    #[builder(setter(into), default = 86400)]
    pub(crate) tb_expired: u32,

    /// Preauth MITM server bind address
    #[cfg(feature = "preauth")]
    #[builder(setter(into), default)]
    pub(crate) pbind: Option<std::net::SocketAddr>,

    /// Preauth MITM server upstream proxy
    #[cfg(feature = "preauth")]
    #[builder(setter(into), default)]
    pub(crate) pupstream: Option<String>,

    /// crate MITM server CA certificate file path
    #[cfg(feature = "preauth")]
    #[builder(setter(into), default)]
    pub(crate) pcert: PathBuf,

    /// Preauth MITM server CA private key file path
    #[cfg(feature = "preauth")]
    #[builder(setter(into), default)]
    pub(crate) pkey: PathBuf,
}
