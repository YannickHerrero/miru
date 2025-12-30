use clap::{Parser, Subcommand};

/// miru - A terminal-native streaming CLI for movies and TV shows
#[derive(Parser)]
#[command(name = "miru")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// First-time setup wizard
    Init,

    /// Manage configuration
    Config {
        /// Show current configuration
        #[arg(long)]
        show: bool,

        /// Set a config value (format: key=value)
        #[arg(long)]
        set: Option<String>,

        /// Reset configuration to defaults
        #[arg(long)]
        reset: bool,
    },

    /// Search for movies and TV shows
    #[command(alias = "s")]
    Search {
        /// Search query
        query: Option<String>,
    },

    /// Play first result, first unwatched episode (coming soon)
    #[command(alias = "p")]
    Play {
        /// Title to play
        query: String,
    },
}
