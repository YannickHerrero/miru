use std::io::{self, Write};

use crate::config::{config_path, load_config, save_config, Config};
use crate::error::Result;
use crate::ui::{App, InitWizard};

/// Run the first-time setup wizard
pub async fn init() -> Result<()> {
    // Check if config already exists
    let config_exists = config_path().exists();

    if config_exists {
        print!("Configuration already exists. Overwrite? [y/N]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Setup cancelled.");
            return Ok(());
        }
    }

    // Run the TUI wizard
    let mut wizard = InitWizard::new(config_exists);
    let completed = wizard.run().await?;

    if completed {
        println!("\nRun 'miru' to start watching.");
    } else {
        println!("\nSetup cancelled.");
    }

    Ok(())
}

/// Handle the config command
pub async fn config(show: bool, set: Option<String>, reset: bool) -> Result<()> {
    if reset {
        if config_path().exists() {
            std::fs::remove_file(config_path())?;
            println!("Configuration reset. Run 'miru init' to set up again.");
        } else {
            println!("No configuration file found.");
        }
        return Ok(());
    }

    if let Some(key_value) = set {
        let parts: Vec<&str> = key_value.splitn(2, '=').collect();
        if parts.len() != 2 {
            println!("Invalid format. Use: --set key=value");
            println!("Available keys: rd_api_key, tmdb_api_key, player_command");
            return Ok(());
        }

        let mut config =
            load_config().unwrap_or_else(|_| Config::new(String::new(), String::new()));

        match parts[0] {
            "rd_api_key" => {
                config.real_debrid.api_key = parts[1].to_string();
            }
            "tmdb_api_key" => {
                config.tmdb.api_key = parts[1].to_string();
            }
            "player_command" => {
                config.player.command = parts[1].to_string();
            }
            _ => {
                println!("Unknown key: {}", parts[0]);
                println!("Available keys: rd_api_key, tmdb_api_key, player_command");
                return Ok(());
            }
        }

        save_config(&config)?;
        println!("Configuration updated.");
        return Ok(());
    }

    if show {
        match load_config() {
            Ok(config) => {
                println!("Configuration file: {}\n", config_path().display());
                println!("[real_debrid]");
                println!(
                    "api_key = \"{}...\"",
                    &config.real_debrid.api_key[..8.min(config.real_debrid.api_key.len())]
                );
                println!("\n[tmdb]");
                if config.tmdb.api_key.is_empty() {
                    println!("api_key = (not configured)");
                } else {
                    println!(
                        "api_key = \"{}...\"",
                        &config.tmdb.api_key[..8.min(config.tmdb.api_key.len())]
                    );
                }
                println!("\n[torrentio]");
                println!("providers = {:?}", config.torrentio.providers);
                println!("quality = \"{}\"", config.torrentio.quality);
                println!("sort = \"{}\"", config.torrentio.sort);
                println!("\n[player]");
                println!("command = \"{}\"", config.player.command);
                println!("args = {:?}", config.player.args);
                println!("\n[ui]");
                println!("theme = \"{}\"", config.ui.theme);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
        return Ok(());
    }

    // Default: show help
    println!("Usage: miru config [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --show         Show current configuration");
    println!("  --set KEY=VAL  Set a configuration value");
    println!("  --reset        Reset configuration to defaults");
    println!();
    println!("Available keys for --set:");
    println!("  rd_api_key      Real-Debrid API key");
    println!("  tmdb_api_key    TMDB API key");
    println!("  player_command  Media player command (default: mpv)");

    Ok(())
}

/// Handle the search command
pub async fn search(query: Option<String>) -> Result<()> {
    let config = load_config()?;
    let mut app = App::new(config);

    if let Some(q) = query {
        app.set_initial_query(&q);
    }

    app.run().await
}

/// Run interactive mode (default)
pub async fn interactive() -> Result<()> {
    let config = match load_config() {
        Ok(c) => c,
        Err(_) => {
            println!("No configuration found. Running setup...\n");
            init().await?;
            return Ok(());
        }
    };

    let mut app = App::new(config);
    app.run().await
}
