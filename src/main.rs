mod api;
mod cli;
mod config;
mod error;
mod history;
mod player;
mod streaming;
mod ui;

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::cli::{Cli, Commands};
use crate::config::PlayerConfig;
use crate::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    let cli = Cli::parse();

    // Check VLC availability early if --vlc flag is passed
    let player_override = if cli.vlc {
        let vlc_config = PlayerConfig::vlc();
        if which::which(&vlc_config.command).is_err() {
            eprintln!("Error: VLC is not installed or not found in PATH.");
            eprintln!("Please install VLC and ensure it's available in your PATH.");
            std::process::exit(1);
        }
        Some(vlc_config)
    } else {
        None
    };

    match cli.command {
        Some(Commands::Init) => {
            cli::commands::init().await?;
        }
        Some(Commands::Config { show, set, reset }) => {
            cli::commands::config(show, set, reset).await?;
        }
        Some(Commands::Search { query }) => {
            cli::commands::search(query, player_override).await?;
        }
        Some(Commands::Play { query: _ }) => {
            println!("Coming soon: direct play feature");
        }
        None => {
            cli::commands::interactive(player_override).await?;
        }
    }

    Ok(())
}
