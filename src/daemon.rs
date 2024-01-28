#[cfg(target_family = "unix")]
use crate::utils;
use crate::{
    args::{self, ServeArgs},
    utils::unix::fix_relative_path,
};
use clap::CommandFactory;
use openai::{arkose::funcaptcha::solver::ArkoseSolver, context::args::Args, proxy, serve::Serve};
use reqwest::impersonate::Impersonate;
use std::{net::IpAddr, ops::Not, path::PathBuf, str::FromStr};
use url::Url;

pub(super) fn serve(mut args: ServeArgs, relative_path: bool) -> anyhow::Result<()> {
    if relative_path {
        fix_relative_path(&mut args);
    }

    if let Some(config_path) = args.config {
        let bytes = std::fs::read(config_path)?;
        let data = String::from_utf8(bytes)?;
        args = toml::from_str::<ServeArgs>(&data)?;
    }

    let arkose_solver = match args.arkose_solver_key.as_ref() {
        Some(client_key) => Some(ArkoseSolver::new(
            args.arkose_solver,
            client_key.clone(),
            args.arkose_solver_url,
            args.arkose_solver_limit,
        )),
        None => None,
    };

    #[cfg(target_os = "linux")]
    if let Some(ref proxies) = args.proxies {
        proxies.iter().for_each(|p| {
            let inner = match p {
                proxy::Proxy::All(v) => v,
                proxy::Proxy::Api(v) => v,
                proxy::Proxy::Auth(v) => v,
                proxy::Proxy::Arkose(v) => v,
            };

            if let proxy::InnerProxy::IPv6Subnet(cidr) = inner {
                utils::unix::sysctl_ipv6_no_local_bind();
                utils::unix::sysctl_route_add_ipv6_subnet(cidr);
            }
        });
    }

    // Set the log level
    std::env::set_var("RUST_LOG", args.level);

    let builder = Args::builder()
        .bind(args.bind)
        .fastest_dns(args.fastest_dns)
        .proxies(args.proxies.unwrap_or_default())
        .enable_direct(args.enable_direct)
        .cookie_store(args.cookie_store)
        .tcp_keepalive(args.tcp_keepalive)
        .no_keepalive(args.no_keepalive)
        .pool_idle_timeout(args.pool_idle_timeout)
        .timeout(args.timeout)
        .connect_timeout(args.connect_timeout)
        .concurrent_limit(args.concurrent_limit)
        .tls_cert(args.tls_cert)
        .tls_key(args.tls_key)
        .auth_key(args.auth_key)
        .visitor_email_whitelist(args.visitor_email_whitelist)
        .cf_site_key(args.cf_site_key)
        .cf_secret_key(args.cf_secret_key)
        .disable_ui(args.disable_webui)
        .arkose_endpoint(args.arkose_endpoint)
        .arkose_gpt3_har_dir(args.arkose_gpt3_har_dir)
        .arkose_gpt4_har_dir(args.arkose_gpt4_har_dir)
        .arkose_auth_har_dir(args.arkose_auth_har_dir)
        .arkose_platform_har_dir(args.arkose_platform_har_dir)
        .arkose_gpt3_experiment(args.arkose_gpt3_experiment)
        .arkose_gpt3_experiment_solver(args.arkose_gpt3_experiment_solver)
        .arkose_har_upload_key(args.arkose_har_upload_key)
        .arkose_solver(arkose_solver)
        .enable_file_proxy(args.enable_file_proxy)
        .enable_arkose_proxy(args.enable_arkose_proxy)
        .pbind(args.pbind)
        .pupstream(args.pupstream)
        .pcert(args.pcert)
        .pkey(args.pkey);

    #[cfg(feature = "limit")]
    let builder = builder
        .tb_enable(args.tb_enable)
        .tb_strategy(args.tb_strategy)
        .tb_capacity(args.tb_capacity)
        .tb_fill_rate(args.tb_fill_rate)
        .tb_expired(args.tb_expired);

    // Parse the impersonate user agents
    if let Some(impersonate_list) = args.impersonate_uas {
        let mut impersonate_uas: Vec<Impersonate> = Vec::new();
        for ua in impersonate_list {
            match Impersonate::from_str(ua.as_str()) {
                Ok(impersonate) => {
                    impersonate_uas.push(impersonate);
                }
                Err(_) => {
                    let mut cmd = args::cmd::Opt::command();
                    cmd.error(
                        clap::error::ErrorKind::ArgumentConflict,
                        &format!("Unsupport impersonate user agent: {}", ua),
                    )
                    .exit();
                }
            }
        }

        let args = builder.impersonate_uas(impersonate_uas).build();
        Serve::new(args).run()
    } else {
        Serve::new(builder.build()).run()
    }
}

#[cfg(target_family = "unix")]
pub(super) fn serve_start(mut args: ServeArgs) -> anyhow::Result<()> {
    use crate::utils::unix::{check_root, get_pid};
    use daemonize::Daemonize;
    use std::{
        fs::{File, Permissions},
        os::unix::prelude::PermissionsExt,
    };

    check_root();

    if let Some(pid) = get_pid() {
        println!("Ninja is already running with pid: {}", pid);
        return Ok(());
    }

    let pid_file = File::create(utils::unix::PID_PATH)?;
    pid_file.set_permissions(Permissions::from_mode(0o755))?;

    let stdout = File::create(utils::unix::DEFAULT_STDOUT_PATH)?;
    stdout.set_permissions(Permissions::from_mode(0o755))?;

    let stderr = File::create(utils::unix::DEFAULT_STDERR_PATH)?;
    stdout.set_permissions(Permissions::from_mode(0o755))?;

    let mut daemonize = Daemonize::new()
        .pid_file(utils::unix::PID_PATH) // Every method except `new` and `start`
        .chown_pid_file(true) // is optional, see `Daemonize` documentation
        .working_directory(utils::unix::DEFAULT_WORK_DIR) // for default behaviour.
        .umask(0o777) // Set umask, `0o027` by default.
        .stdout(stdout) // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr) // Redirect stderr to `/tmp/daemon.err`.
        .privileged_action(|| "Executed before drop privileges");

    if let Ok(user) = std::env::var("SUDO_USER") {
        if let Ok(Some(real_user)) = nix::unistd::User::from_name(&user) {
            daemonize = daemonize
                .user(real_user.name.as_str())
                .group(real_user.gid.as_raw());
        }
    }

    fix_relative_path(&mut args);

    if let Some(err) = daemonize.start().err() {
        eprintln!("Error: {err}")
    }

    serve(args, false)
}

#[cfg(target_family = "unix")]
pub(super) fn serve_stop() -> anyhow::Result<()> {
    use crate::utils::unix::{check_root, get_pid};
    use nix::sys::signal;
    use nix::unistd::Pid;

    check_root();

    if let Some(pid) = get_pid() {
        let pid = pid.parse::<i32>()?;
        for _ in 0..360 {
            if signal::kill(Pid::from_raw(pid), signal::SIGINT).is_err() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1))
        }
        let _ = std::fs::remove_file(utils::unix::PID_PATH);
    }

    Ok(())
}

#[cfg(target_family = "unix")]
pub(super) fn serve_restart(args: ServeArgs) -> anyhow::Result<()> {
    use crate::utils::unix::check_root;
    check_root();
    serve_stop()?;
    serve_start(args)
}

#[cfg(target_family = "unix")]
pub(super) fn serve_status() -> anyhow::Result<()> {
    use crate::utils::unix::get_pid;
    match get_pid() {
        Some(pid) => println!("Ninja is running with pid: {}", pid),
        None => println!("Ninja is not running"),
    }
    Ok(())
}

#[cfg(target_family = "unix")]
pub(super) fn serve_log() -> anyhow::Result<()> {
    use std::{
        fs::File,
        io::{self, BufRead},
        path::Path,
    };

    fn read_and_print_file(file_path: &Path, placeholder: &str) -> anyhow::Result<()> {
        if !file_path.exists() {
            return Ok(());
        }

        // Check if the file is empty before opening it
        let metadata = std::fs::metadata(file_path)?;
        if metadata.len() == 0 {
            return Ok(());
        }

        let file = File::open(file_path)?;
        let reader = io::BufReader::new(file);
        let mut start = true;

        for line in reader.lines() {
            if let Ok(content) = line {
                if start {
                    start = false;
                    println!("{placeholder}");
                }
                println!("{}", content);
            } else if let Err(err) = line {
                eprintln!("Error reading line: {}", err);
            }
        }

        Ok(())
    }

    let stdout_path = Path::new(utils::unix::DEFAULT_STDOUT_PATH);
    read_and_print_file(stdout_path, "STDOUT>")?;

    let stderr_path = Path::new(utils::unix::DEFAULT_STDERR_PATH);
    read_and_print_file(stderr_path, "STDERR>")?;

    Ok(())
}

pub(super) fn generate_template(out: Option<PathBuf>) -> anyhow::Result<()> {
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
        std::env::current_dir()?.join("serve.toml")
    };

    let args = args::ServeArgs {
        bind: Some("0.0.0.0:7999".parse()?),
        concurrent_limit: 65535,
        timeout: 600,
        connect_timeout: 60,
        tcp_keepalive: 60,
        tb_strategy: "mem".to_string(),
        tb_enable: false,
        tb_capacity: 60,
        tb_fill_rate: 1,
        tb_expired: 86400,
        cookie_store: true,
        pool_idle_timeout: 90,
        level: "info".to_owned(),
        pcert: PathBuf::from("ca/cert.crt"),
        pkey: PathBuf::from("ca/key.pem"),
        arkose_gpt3_experiment: false,
        enable_file_proxy: false,
        proxies: Some(vec![
            proxy::Proxy::try_from(("all", "socks5://127.0.0.1:8888".parse::<Url>()?))?,
            proxy::Proxy::try_from(("all", "http://127.0.0.1:8889".parse::<Url>()?))?,
            proxy::Proxy::try_from(("api", "192.168.1.1".parse::<IpAddr>()?))?,
            proxy::Proxy::try_from(("api", cidr::Ipv6Cidr::from_str("2001:db8::/32")?))?,
        ]),
        ..args::ServeArgs::default()
    };

    let write = |out: PathBuf, args: ServeArgs| -> anyhow::Result<()> {
        #[cfg(target_family = "unix")]
        {
            use std::fs::Permissions;
            use std::os::unix::prelude::PermissionsExt;
            std::fs::File::create(&out)?.set_permissions(Permissions::from_mode(0o755))?;
        }

        #[cfg(target_family = "windows")]
        std::fs::File::create(&out)?;

        Ok(std::fs::write(out, toml::to_string_pretty(&args)?)?)
    };

    if !out.exists() {
        write(out, args)?;
    }
    Ok(())
}
