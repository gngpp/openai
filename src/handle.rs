use std::{ops::Not, path::PathBuf};

use crate::args::ServeArgs;

pub(super) async fn run_serve(mut args: ServeArgs) -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::default());
    if let Some(config_path) = args.config {
        log::info!("Using config file: {}", config_path.display());
        let bytes = tokio::fs::read(config_path).await?;
        let data = String::from_utf8(bytes)?;
        args = toml::from_str::<ServeArgs>(&data)?;
    }
    let mut builder = openai::serve::LauncherBuilder::default();
    let builder = builder
        .host(
            args.host
                .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0))),
        )
        .port(args.port.unwrap_or(7999))
        .proxies(args.proxies.unwrap_or_default())
        .api_prefix(args.api_prefix)
        .tls_keypair(None)
        .tcp_keepalive(args.tcp_keepalive.max(60))
        .timeout(args.timeout.max(600))
        .connect_timeout(args.connect_timeout.max(60))
        .workers(args.workers.max(1))
        .cf_site_key(args.cf_site_key)
        .cf_secret_key(args.cf_secret_key);

    #[cfg(feature = "limit")]
    let builder = builder
        .tb_enable(args.tb_enable)
        .tb_store_strategy(args.tb_store_strategy)
        .tb_redis_url(args.tb_redis_url)
        .tb_capacity(args.tb_capacity.max(60))
        .tb_fill_rate(args.tb_fill_rate.max(1))
        .tb_expired(args.tb_expired.max(86400));

    #[cfg(feature = "sign")]
    let mut builder = builder.sign_secret_key(args.sign_secret_key);

    if args.tls_key.is_some() && args.tls_cert.is_some() {
        builder = builder.tls_keypair(Some((args.tls_cert.unwrap(), args.tls_key.unwrap())));
    }
    builder.build()?.run().await
}

pub(super) async fn generate_template(cover: bool, out: Option<PathBuf>) -> anyhow::Result<()> {
    let out = if let Some(out) = out {
        match out.is_dir() {
            false => {
                if let Some(parent) = out.parent() {
                    if parent.exists().not() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                }
            }
            true => anyhow::bail!("{} not a file", out.display()),
        };
        out
    } else {
        std::env::current_dir()?.join("opengpt-serve.toml")
    };

    let template = "host=\"0.0.0.0\"\nport=7999\nworkers=1\n#proxies=[]\ntimeout=600\nconnect_timeout=60\ntcp_keepalive=60\n#tls_cert=\n#tls_key=\n#api_prefix=\ntb_enable=false\ntb_store_strategy=\"mem\"\ntb_redis_url=[\"redis://127.0.0.1:6379\"]\ntb_capacity=60\ntb_fill_rate=1\ntb_expired=86400\n#sign_secret_key=\n#cf_site_key=\n#cf_secret_key=\n";

    if cover {
        #[cfg(target_family = "unix")]
        {
            use std::fs::Permissions;
            use std::os::unix::prelude::PermissionsExt;
            tokio::fs::File::create(&out)
                .await?
                .set_permissions(Permissions::from_mode(0o755))
                .await?;
        }

        #[cfg(target_family = "windows")]
        tokio::fs::File::create(&out).await?;

        Ok(tokio::fs::write(out, template).await?)
    } else {
        Ok(())
    }
}
