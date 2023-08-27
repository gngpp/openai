use std::sync::Once;

use crate::{auth::AuthClient, info};
use reqwest::Client;
use std::sync::RwLock;

use super::{load_balancer, Launcher};

static mut ENV: Option<Env> = None;
pub(super) static ENV_HOLDER: EnvWrapper = EnvWrapper(Once::new());

pub(super) struct Env {
    api_client_load: load_balancer::ClientLoadBalancer<Client>,
    auth_client_load: load_balancer::ClientLoadBalancer<AuthClient>,
    share_puid: RwLock<Option<String>>,
    arkose_token_endpoint: Option<String>,
}

impl Env {
    fn new(args: &super::Launcher) -> Self {
        let puid = if let Some(puid) = args.puid.as_ref() {
            info!("Using PUID: {puid}");
            Some(puid.to_owned())
        } else {
            None
        };
        Env {
            api_client_load: load_balancer::ClientLoadBalancer::<Client>::new_api_client(args)
                .expect("Failed to initialize the requesting client"),
            auth_client_load: load_balancer::ClientLoadBalancer::<AuthClient>::new_auth_client(
                args,
            )
            .expect("Failed to initialize the requesting oauth client"),
            share_puid: RwLock::new(puid),
            arkose_token_endpoint: args.arkose_token_endpoint.clone(),
        }
    }

    pub fn load_api_client(&self) -> Client {
        self.api_client_load.next()
    }

    pub fn load_auth_client(&self) -> AuthClient {
        self.auth_client_load.next()
    }

    pub fn get_arkose_token_endpoint(&self) -> Option<&str> {
        self.arkose_token_endpoint.as_deref()
    }

    pub fn get_share_puid(&self) -> Option<String> {
        let lock = self.share_puid.read().unwrap();
        lock.clone()
    }

    pub fn set_share_puid(&self, puid: Option<String>) {
        let mut lock = self.share_puid.write().unwrap();
        *lock = puid;
    }
}

pub(super) struct EnvWrapper(Once);

impl EnvWrapper {
    pub fn init(&self, args: &Launcher) {
        // Use Once to guarantee initialization only once
        self.0.call_once(|| unsafe { ENV = Some(Env::new(args)) });
    }

    pub fn get_instance(&self) -> &Env {
        unsafe {
            ENV.as_ref()
                .expect("Runtime Env component is not initialized")
        }
    }
}
