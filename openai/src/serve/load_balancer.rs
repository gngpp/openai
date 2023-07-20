use reqwest::{browser::ChromeVersion, Client};
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use crate::auth::AuthClient;
use crate::{auth, info, HEADER_UA};

pub(super) struct ClientLoadBalancer<T: Clone> {
    clients: Vec<T>,
    index: AtomicUsize,
}

impl<T: Clone> ClientLoadBalancer<T> {
    pub(super) fn new_auth_client(
        args: &super::Launcher,
    ) -> anyhow::Result<ClientLoadBalancer<AuthClient>> {
        let build = |proxy_url: Option<String>| -> AuthClient {
            if let Some(ref url) = proxy_url {
                info!("Add {url} to the Auth load balancing client proxy pool");
            }
            // auth client
            let auth_client = auth::AuthClientBuilder::builder()
                .user_agent(HEADER_UA)
                .chrome_builder(ChromeVersion::V108)
                .timeout(Duration::from_secs((args.timeout + 1) as u64))
                .connect_timeout(Duration::from_secs((args.connect_timeout + 1) as u64))
                .pool_max_idle_per_host(args.workers)
                .cookie_store(true)
                .proxy(proxy_url)
                .build();
            auth_client
        };

        let mut clients = Vec::new();

        if args.proxies.is_empty() {
            clients.push(build(None));
        } else {
            for proxy in args.proxies.clone() {
                clients.push(build(Some(proxy)));
            }
        }

        Ok(ClientLoadBalancer {
            clients,
            index: AtomicUsize::new(0),
        })
    }

    pub(super) fn new_api_client(
        args: &super::Launcher,
    ) -> anyhow::Result<ClientLoadBalancer<Client>> {
        let build = |proxy_url: Option<String>| -> reqwest::Client {
            let mut client_builder = reqwest::Client::builder();
            if let Some(url) = proxy_url {
                info!("Add {url} to the API load balancing client proxy pool");
                let proxy = reqwest::Proxy::all(url).unwrap();
                client_builder = client_builder.proxy(proxy)
            }

            // api client
            let client = client_builder
                .user_agent(HEADER_UA)
                .chrome_builder(ChromeVersion::V110)
                .tcp_keepalive(Some(Duration::from_secs((args.tcp_keepalive + 1) as u64)))
                .timeout(Duration::from_secs((args.timeout + 1) as u64))
                .connect_timeout(Duration::from_secs((args.connect_timeout + 1) as u64))
                .pool_max_idle_per_host(args.workers)
                .cookie_store(true)
                .build()
                .expect("Failed to build API client");
            client
        };

        let mut clients = Vec::new();

        if args.proxies.is_empty() {
            clients.push(build(None));
        } else {
            for proxy in args.proxies.clone() {
                clients.push(build(Some(proxy)));
            }
        }
        let load = ClientLoadBalancer {
            clients,
            index: AtomicUsize::new(0),
        };
        Ok(load)
    }

    pub(super) fn next(&self) -> &T {
        let len = self.clients.len();
        let mut old = self.index.load(Ordering::Relaxed);
        let mut new;
        loop {
            new = (old + 1) % len;
            match self
                .index
                .compare_exchange_weak(old, new, Ordering::SeqCst, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(x) => old = x,
            }
        }
        &self.clients[old]
    }
}
