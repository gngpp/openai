use clap::{command, Parser};
use std::{io::Write, sync::Once};

#[derive(Parser)]
#[clap(author, version, about, arg_required_else_help = true)]
#[command(args_conflicts_with_subcommands = true)]
struct Opt {
    /// Enable debug
    #[clap(long, global = true, value_parser = initialize_log)]
    debug: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Opt::parse();

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
