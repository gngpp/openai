use std::{ops::Not, path::PathBuf};

use crate::{args::ServeArgs, env::fix_relative_path};

#[tokio::main]
pub(super) async fn serve(mut args: ServeArgs, relative_path: bool) -> anyhow::Result<()> {
    env_logger::init_from_env(env_logger::Env::default());

    if relative_path {
        fix_relative_path(&mut args);
    }

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

#[cfg(target_family = "unix")]
pub(super) fn serve_start(mut args: ServeArgs) -> anyhow::Result<()> {
    use crate::env::{self, check_root, get_pid};
    use daemonize::Daemonize;
    use std::{
        fs::{File, Permissions},
        os::unix::prelude::PermissionsExt,
    };

    check_root();

    if let Some(pid) = get_pid() {
        println!("OpenGPT is already running with pid: {}", pid);
        return Ok(());
    }

    let pid_file = File::create(env::PID_PATH).unwrap();
    pid_file.set_permissions(Permissions::from_mode(0o755))?;

    let stdout = File::create(env::DEFAULT_STDOUT_PATH).unwrap();
    stdout.set_permissions(Permissions::from_mode(0o755))?;

    let stderr = File::create(env::DEFAULT_STDERR_PATH).unwrap();
    stdout.set_permissions(Permissions::from_mode(0o755))?;

    let mut daemonize = Daemonize::new()
        .pid_file(env::PID_PATH) // Every method except `new` and `start`
        .chown_pid_file(true) // is optional, see `Daemonize` documentation
        .working_directory(env::DEFAULT_WORK_DIR) // for default behaviour.
        .umask(0o777) // Set umask, `0o027` by default.
        .stdout(stdout) // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr) // Redirect stderr to `/tmp/daemon.err`.
        .privileged_action(|| "Executed before drop privileges");

    match std::env::var("SUDO_USER") {
        Ok(user) => {
            if let Ok(Some(real_user)) = nix::unistd::User::from_name(&user) {
                daemonize = daemonize
                    .user(real_user.name.as_str())
                    .group(real_user.gid.as_raw());
            }
        }
        Err(_) => println!("Could not interpret SUDO_USER"),
    }

    fix_relative_path(&mut args);

    match daemonize.start() {
        Ok(_) => println!("Success, daemonized"),
        Err(e) => eprintln!("Error, {}", e),
    }

    serve(args, false)
}

#[cfg(target_family = "unix")]
pub(super) fn serve_stop() -> anyhow::Result<()> {
    use crate::env::{self, check_root, get_pid};
    use nix::sys::signal;
    use nix::unistd::Pid;

    check_root();

    if let Some(pid) = get_pid() {
        let pid = pid.parse::<i32>()?;
        if let Err(_) = nix::sys::signal::kill(Pid::from_raw(pid), signal::SIGINT) {
            println!("OpenGPT is not running");
        }
        let _ = std::fs::remove_file(env::PID_PATH);
    } else {
        println!("OpenGPT is not running")
    };

    Ok(())
}

#[cfg(target_family = "unix")]
pub(super) fn serve_restart(args: ServeArgs) -> anyhow::Result<()> {
    use crate::env::check_root;

    check_root();
    println!("Restarting OpenGPT...");
    serve_stop()?;
    serve_start(args)
}

#[cfg(target_family = "unix")]
pub(super) fn serve_status() -> anyhow::Result<()> {
    use crate::env::get_pid;
    if let Some(pid) = get_pid() {
        println!("OpenGPT is running with pid: {}", pid);
    } else {
        println!("OpenGPT is not running")
    }
    Ok(())
}

#[cfg(target_family = "unix")]
pub(super) fn serve_log() -> anyhow::Result<()> {
    use crate::env;
    use std::{
        fs::File,
        io::{self, BufRead},
        path::Path,
    };

    let path = Path::new(env::DEFAULT_STDERR_PATH); // 请用你的日志文件路径替换
    let file = File::open(&path)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        match line {
            Ok(content) => println!("{}", content),
            Err(err) => eprintln!("Error reading line: {}", err),
        }
    }
    Ok(())
}

pub(super) fn generate_template(cover: bool, out: Option<PathBuf>) -> anyhow::Result<()> {
    let out = if let Some(out) = out {
        match out.is_dir() {
            false => {
                if let Some(parent) = out.parent() {
                    if parent.exists().not() {
                        std::fs::create_dir_all(parent)?;
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
            std::fs::File::create(&out)?.set_permissions(Permissions::from_mode(0o755))?;
        }

        #[cfg(target_family = "windows")]
        std::fs::File::create(&out)?;

        Ok(std::fs::write(out, template)?)
    } else {
        Ok(())
    }
}
