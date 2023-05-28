use clap::{command, Parser, Subcommand};
use std::{io::Write, sync::Once};

pub mod openai;

#[derive(Parser, Debug)]
#[clap(author, version, about, arg_required_else_help = true)]
struct Opt {
    /// Enable debug
    #[clap(long, global = true, value_parser = initialize_log)]
    debug: bool,

    /// Proxy. Format: protocol://user:pass@ip:port
    #[clap(short, long)]
    proxy: Option<String>,

    /// OpenAI gpt-3.5-turbo chat api, Note: OpenAI will bill you
    #[clap(short = 'T', long, env = "OPENGPT_TOKEN")]
    turbo: Option<String>,

    /// OpenAI account email, Format: gngppz@gmail.com
    #[arg(
        short = 'E',
        long,
        env = "OPENGPT_EMAIL",
        requires = "password",
    )]
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
        #[clap(short = 'H', long, default_value = "0.0.0.0", value_parser = parser_host)]
        host: std::net::IpAddr,
        /// Server Listen port
        #[clap(short = 'P', long, default_value = "7999", value_parser = parser_port_in_range)]
        port: u16,
    },
    /// Start configuration
    Config {
        /// Specify an access token storage file path
        #[clap(short, long)]
        token_storage: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::parse();
    println!("{:#?}", opt);
    println!("Hello, world!");
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

// port range parser
fn parser_port_in_range(s: &str) -> anyhow::Result<u16> {
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

// address parser
fn parser_host(s: &str) -> anyhow::Result<std::net::IpAddr> {
    let addr = s
        .parse::<std::net::IpAddr>()
        .map_err(|_| anyhow::anyhow!(format!("`{}` isn't a ip address", s)))?;
    Ok(addr)
}
