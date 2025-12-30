mod api;
mod cli;
mod config;
mod error;
mod player;
mod ui;

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::cli::{Cli, Commands};
use crate::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")))
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Init) => {
            cli::commands::init().await?;
        }
        Some(Commands::Config { show, set, reset }) => {
            cli::commands::config(show, set, reset).await?;
        }
        Some(Commands::Search { query }) => {
            cli::commands::search(query).await?;
        }
        Some(Commands::Play { query: _ }) => {
            println!("Coming soon: direct play feature");
        }
        None => {
            cli::commands::interactive().await?;
        }
    }

    Ok(())
}
