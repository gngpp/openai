use std::sync::Arc;

use args::SubCommands;
use clap::Parser;

pub mod account;
pub mod args;
pub mod env;
pub mod handle;
pub mod prompt;
pub mod ui;
pub mod util;

fn main() -> anyhow::Result<()> {
    let opt = args::Opt::parse();
    std::env::set_var("RUST_LOG", opt.level);
    match opt.command {
        Some(command) => match command {
            SubCommands::Account => {
                prompt::account_prompt()?;
            }
            SubCommands::Config {
                workdir: _,
                unofficial_api: _,
                unofficial_proxy: _,
            } => {}
            SubCommands::Serve(commands) => match commands {
                args::ServeSubcommand::Run(args) => handle::serve(args, true)?,
                #[cfg(target_family = "unix")]
                args::ServeSubcommand::Stop => handle::serve_stop()?,
                #[cfg(target_family = "unix")]
                args::ServeSubcommand::Start(args) => handle::serve_start(args)?,
                #[cfg(target_family = "unix")]
                args::ServeSubcommand::Restart(args) => handle::serve_restart(args)?,
                #[cfg(target_family = "unix")]
                args::ServeSubcommand::Status => handle::serve_status()?,
                #[cfg(target_family = "unix")]
                args::ServeSubcommand::Log => handle::serve_log()?,
                args::ServeSubcommand::GT { cover, out } => {
                    handle::generate_template(cover, out)?;
                }
            },
        },
        None => main_ui()?,
    }
    Ok(())
}

#[tokio::main]
async fn main_ui() -> anyhow::Result<()> {
    let (sync_io_tx, mut sync_io_rx) = tokio::sync::mpsc::channel::<ui::io::IoEvent>(100);

    // We need to share the App between thread
    let app = Arc::new(tokio::sync::Mutex::new(ui::app::App::new(
        sync_io_tx.clone(),
    )));
    let app_ui = Arc::clone(&app);

    // Configure log
    tui_logger::init_logger(log::LevelFilter::Debug)?;
    tui_logger::set_default_level(log::LevelFilter::Debug);

    // Handle IO in a specifc thread
    tokio::spawn(async move {
        let mut handler = ui::io::handler::IoAsyncHandler::new(app);
        while let Some(io_event) = sync_io_rx.recv().await {
            handler.handle_io_event(io_event).await;
        }
    });

    ui::start_ui(&app_ui).await
}
