use super::{
    args::Args,
    arkose::{
        har::{HarProvider, HAR},
        ArkoseVersionContext,
    },
    preauth::PreauthCookieProvider,
    CfTurnstile, Context, CTX,
};
use crate::{arkose, client::ClientRoundRobinBalancer, error};
use std::{collections::HashMap, sync::RwLock};

/// Use Once to guarantee initialization only once
pub fn init(args: Args) {
    if let Some(_) = CTX.set(init_context(args.clone())).err() {
        error!("Failed to initialize context");
    };

    if let Some(_) = HAR.set(RwLock::new(init_har_provider(args))).err() {
        error!("Failed to initialize har provider");
    };
}

/// Get the program context
pub fn instance() -> &'static Context {
    CTX.get_or_init(|| init_context(Args::builder().build()))
}

/// Init the program context
fn init_context(args: Args) -> Context {
    Context {
        api_client: ClientRoundRobinBalancer::new_client(&args)
            .expect("Failed to initialize the requesting client"),
        auth_client: ClientRoundRobinBalancer::new_auth_client(&args)
            .expect("Failed to initialize the requesting oauth client"),
        arkose_client: ClientRoundRobinBalancer::new_arkose_client(&args)
            .expect("Failed to initialize the requesting arkose client"),
        preauth_provider: args.pbind.is_some().then(|| PreauthCookieProvider::new()),
        arkose_endpoint: args.arkose_endpoint,
        arkose_context: ArkoseVersionContext::new(),
        arkose_solver: args.arkose_solver,
        arkose_har_upload_key: args.arkose_har_upload_key,
        arkose_gpt3_experiment: args.arkose_gpt3_experiment,
        arkose_gpt3_experiment_solver: args.arkose_gpt3_experiment_solver,
        arkose_solver_tguess_endpoint: args.arkose_solver_tguess_endpoint,
        arkose_solver_image_dir: args.arkose_solver_image_dir,
        enable_file_proxy: args.enable_file_proxy,
        auth_key: args.auth_key,
        visitor_email_whitelist: args.visitor_email_whitelist,
        cf_turnstile: args.cf_site_key.and_then(|site_key| {
            args.cf_secret_key.map(|secret_key| CfTurnstile {
                site_key,
                secret_key,
            })
        }),
    }
}

fn init_har_provider(args: Args) -> HashMap<arkose::Type, HarProvider> {
    let gpt3_har_provider = HarProvider::new(
        arkose::Type::GPT3,
        args.arkose_gpt3_har_dir.as_ref(),
        "gpt3",
    );
    let gpt4_har_provider = HarProvider::new(
        arkose::Type::GPT4,
        args.arkose_gpt4_har_dir.as_ref(),
        "gpt4",
    );
    let auth_har_provider = HarProvider::new(
        arkose::Type::Auth,
        args.arkose_auth_har_dir.as_ref(),
        "auth",
    );
    let platform_har_provider = HarProvider::new(
        arkose::Type::Platform,
        args.arkose_platform_har_dir.as_ref(),
        "platform",
    );
    let signup_har_provider = HarProvider::new(
        arkose::Type::SignUp,
        args.arkose_platform_har_dir.as_ref(),
        "signup",
    );

    let mut har_map = HashMap::with_capacity(5);
    har_map.insert(arkose::Type::GPT3, gpt3_har_provider);
    har_map.insert(arkose::Type::GPT4, gpt4_har_provider);
    har_map.insert(arkose::Type::Auth, auth_har_provider);
    har_map.insert(arkose::Type::Platform, platform_har_provider);
    har_map.insert(arkose::Type::SignUp, signup_har_provider);

    har_map
}
