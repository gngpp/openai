pub mod args;
pub mod arkose;
pub mod init;
mod preauth;

use self::preauth::PreauthCookieProvider;
use crate::{
    arkose::funcaptcha::solver::ArkoseSolver, auth::AuthClient, client::ClientRoundRobinBalancer,
};
use reqwest::Client;
use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub const WORKER_DIR: &str = ".ninja";
// Program context
static CTX: OnceLock<Context> = OnceLock::new();

#[macro_export]
macro_rules! with_context {
    ($method:ident) => {{
        crate::context::init::instance().$method()
    }};
    ($method:ident, $($arg:expr),*) => {{
        crate::context::init::instance().$method($($arg),*)
    }};
    () => {
        crate::context::init::instance()
    };
}

pub fn init(args: args::Args) {
    init::init(args);
}

pub struct CfTurnstile {
    pub site_key: String,
    pub secret_key: String,
}

pub struct Context {
    /// Requesting client
    api_client: ClientRoundRobinBalancer,
    /// Requesting oauth client
    auth_client: ClientRoundRobinBalancer,
    /// Requesting arkose client
    arkose_client: ClientRoundRobinBalancer,
    /// Arkoselabs context
    arkose_context: arkose::ArkoseVersionContext<'static>,
    /// arkoselabs solver
    arkose_solver: Option<ArkoseSolver>,
    /// Enable files proxy
    enable_file_proxy: bool,
    /// Login auth key
    auth_key: Option<String>,
    /// visitor_email_whitelist
    visitor_email_whitelist: Option<Vec<String>>,
    /// Cloudflare Turnstile
    cf_turnstile: Option<CfTurnstile>,
    /// Arkose endpoint
    arkose_endpoint: Option<String>,
    /// Websocket endpoint
    websocket_endpoint: Option<String>,
    /// Enable Arkose GPT-3.5 experiment
    arkose_gpt3_experiment: bool,
    /// Enable Arkose GPT-3.5 experiment solver
    arkose_gpt3_experiment_solver: bool,
    /// Arkose solver tguess endpoint
    arkose_solver_tguess_endpoint: Option<String>,
    /// Arkose solver image store directory
    arkose_solver_image_dir: Option<PathBuf>,
    /// PreAuth cookie cache
    preauth_provider: Option<PreauthCookieProvider>,
}

impl Context {
    /// Get the reqwest client
    pub fn api_client(&self) -> Client {
        self.api_client.next().into()
    }

    /// Get the reqwest auth client
    pub fn auth_client(&self) -> AuthClient {
        self.auth_client.next().into()
    }

    /// Get the reqwest arkose client
    pub fn arkose_client(&self) -> Client {
        self.arkose_client.next().into()
    }

    /// Get the arkoselabs solver
    pub fn arkose_solver(&self) -> Option<&ArkoseSolver> {
        self.arkose_solver.as_ref()
    }

    /// Cloudflare Turnstile config
    pub fn cf_turnstile(&self) -> Option<&CfTurnstile> {
        self.cf_turnstile.as_ref()
    }

    /// Arkoselabs endpoint
    pub fn arkose_endpoint(&self) -> Option<&str> {
        self.arkose_endpoint.as_deref()
    }

    /// Login auth key
    pub fn auth_key(&self) -> Option<&str> {
        self.auth_key.as_deref()
    }

    /// Push a preauth cookie
    #[cfg(feature = "preauth")]
    pub fn push_preauth_cookie(&self, value: &str, max_age: Option<u32>) {
        self.preauth_provider
            .as_ref()
            .map(|p| p.push(value, max_age));
    }

    /// Pop a preauth cookie
    #[cfg(feature = "preauth")]
    pub fn pop_preauth_cookie(&self) -> Option<String> {
        self.preauth_provider.as_ref().map(|p| p.get()).flatten()
    }

    /// Get the arkose gpt3 experiment
    pub fn arkose_gpt3_experiment(&self) -> bool {
        self.arkose_gpt3_experiment
    }

    /// Enable file proxy
    pub fn enable_file_proxy(&self) -> bool {
        self.enable_file_proxy
    }

    /// Get the visitor email whitelist
    pub fn visitor_email_whitelist(&self) -> Option<&[String]> {
        self.visitor_email_whitelist.as_deref()
    }

    /// Get the arkose gpt3 experiment solver
    pub fn arkose_gpt3_experiment_solver(&self) -> bool {
        self.arkose_gpt3_experiment_solver
    }

    /// Get websocket endpoint
    pub fn websocket_endpoint(&self) ->  Option<&str> {
        self.websocket_endpoint.as_deref()
    }

    /// Get the arkose context
    pub fn arkose_context(&self) -> &arkose::ArkoseVersionContext<'static> {
        &self.arkose_context
    }

    /// Get the arkose solver tguess endpoint, Example: https://tguess.arkoselabs.com
    pub fn arkose_solver_tguess_endpoint(&self) -> Option<&str> {
        self.arkose_solver_tguess_endpoint.as_deref()
    }

    /// Get the arkose solver image store directory, Example: /home/user/.ninja/image
    pub fn arkose_solver_image_dir(&self) -> Option<&Path> {
        self.arkose_solver_image_dir.as_deref()
    }
}
