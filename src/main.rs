use anyhow::Context;
use clap::{Parser, Subcommand};
use std::{io::Write, path::PathBuf, sync::Once};

pub mod util;

#[derive(Parser, Debug)]
#[clap(author, version, about, arg_required_else_help = true)]
struct Opt {
    /// Enable debug
    #[clap(long, global = true, env = "OPENGPT_DEBUG", value_parser = initialize_log)]
    debug: bool,

    /// HTTP Proxy. Format: protocol://user:pass@ip:port
    #[clap(short, long, env = "OPENGPT_PROXY", value_parser = parse_proxy_url)]
    proxy: Option<url::Url>,

    /// OpenAI gpt-3.5-turbo chat api, Note: OpenAI will bill you
    #[clap(short, long, env = "OPENGPT_TURBO")]
    turbo: bool,

    /// OpenAI account email, Format: gngppz@gmail.com
    #[arg(short = 'E', long, env = "OPENGPT_EMAIL", requires = "password")]
    email: Option<String>,

    /// OpenAI account password
    #[arg(short = 'W', long, env = "OPENGPT_PASSWORD", requires = "email")]
    password: Option<String>,

    #[clap(subcommand)]
    command: Option<SubCommands>,
}

#[derive(Subcommand, Debug)]
enum SubCommands {
    /// Start proxy server
    Server {
        /// Server Listen host
        #[clap(short = 'H', long, default_value = "0.0.0.0", value_parser = parse_host)]
        host: std::net::IpAddr,
        /// Server Listen port
        #[clap(short = 'P', long, default_value = "7999", value_parser = parse_port_in_range)]
        port: u16,
    },
    /// Setting configuration
    Config {
        /// Working directory, refresh_token will be stored in there if specified
        #[clap(short = 'W', long)]
        workdir: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _opt = Opt::parse();
    Ok(())
}

static INIT: Once = Once::new();

fn initialize_log(s: &str) -> anyhow::Result<bool> {
    let debug = s.parse::<bool>()?;
    match debug {
        true => std::env::set_var("RUST_LOG", "DEBUG"),
        false => std::env::set_var("RUST_LOG", "INFO"),
    };
    INIT.call_once(|| {
        env_logger::builder()
            .format(|buf, record| {
                writeln!(
                    buf,
                    "{} {}: {}",
                    record.level(),
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    record.args()
                )
            })
            .init();
    });
    Ok(debug)
}

const PORT_RANGE: std::ops::RangeInclusive<usize> = 1024..=65535;

// port range parse
fn parse_port_in_range(s: &str) -> anyhow::Result<u16> {
    let port: usize = s
        .parse()
        .map_err(|_| anyhow::anyhow!(format!("`{}` isn't a port number", s)))?;
    if PORT_RANGE.contains(&port) {
        return Ok(port as u16);
    }
    anyhow::bail!(format!(
        "Port not in range {}-{}",
        PORT_RANGE.start(),
        PORT_RANGE.end()
    ))
}

// address parse
fn parse_host(s: &str) -> anyhow::Result<std::net::IpAddr> {
    let addr = s
        .parse::<std::net::IpAddr>()
        .map_err(|_| anyhow::anyhow!(format!("`{}` isn't a ip address", s)))?;
    Ok(addr)
}

// proxy proto
fn parse_proxy_url(proxy_url: &str) -> anyhow::Result<url::Url> {
    let url = url::Url::parse(proxy_url)
        .context("The Proxy Url format must be `protocol://user:pass@ip:port`")?;
    let protocol = url.scheme().to_string();
    match protocol.as_str() {
        "http" | "https" | "sockt5" => Ok(url),
        _ => anyhow::bail!("Unsupported protocol: {}", protocol),
    }
}
