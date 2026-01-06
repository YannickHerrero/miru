use std::io::{self, Write};

use crate::api::RealDebridClient;
use crate::config::{config_path, load_config, save_config, Config};
use crate::error::Result;
use crate::ui::App;

const ASCII_ART: &str = r#"
          _              
  _ __ ___ (_)_ __ _   _  
 | '_ ` _ \| | '__| | | | 
 | | | | | | | |  | |_| | 
 |_| |_| |_|_|_|   \__,_| 
                          
"#;

/// Run the first-time setup wizard
pub async fn init() -> Result<()> {
    println!("{}", ASCII_ART);
    println!("Welcome to miru - your terminal streaming companion!\n");

    // Check if config already exists
    if config_path().exists() {
        print!("Configuration already exists. Overwrite? [y/N]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Setup cancelled.");
            return Ok(());
        }
    }

    // Prompt for Real-Debrid API key (optional)
    println!("miru supports two streaming modes:\n");
    println!("  1. Direct P2P Streaming (free, no account needed)");
    println!("     - Download torrents directly to your device");
    println!("     - No Real-Debrid account required\n");
    println!("  2. Real-Debrid Cached (faster, requires account)");
    println!("     - Access cached torrents on Real-Debrid servers");
    println!("     - Faster speeds, less bandwidth usage");
    println!("     - Get a free account at: https://real-debrid.com\n");

    print!("Enter your Real-Debrid API key (or press Enter to use direct P2P): ");
    io::stdout().flush()?;

    let mut rd_api_key = String::new();
    io::stdin().read_line(&mut rd_api_key)?;
    let rd_api_key = rd_api_key.trim().to_string();

    if !rd_api_key.is_empty() {
        // Validate the Real-Debrid API key only if one was provided
        print!("Validating Real-Debrid API key... ");
        io::stdout().flush()?;

        let client = RealDebridClient::new(rd_api_key.clone());
        match client.validate_key().await {
            Ok(user) => {
                println!("OK!");
                println!("Logged in as: {}", user.username);
            }
            Err(e) => {
                println!("Failed!");
                println!("Error: {}", e);
                println!("\nPlease check your API key and try again.");
                return Ok(());
            }
        }
    } else {
        println!("Using direct P2P streaming mode.");
    }

    // Prompt for TMDB API key
    println!("\nTo search for movies and TV shows, you need a TMDB API key.");
    println!("Get yours at: https://www.themoviedb.org/settings/api\n");

    print!("Enter your TMDB API key (or press Enter to skip): ");
    io::stdout().flush()?;

    let mut tmdb_api_key = String::new();
    io::stdin().read_line(&mut tmdb_api_key)?;
    let tmdb_api_key = tmdb_api_key.trim().to_string();

    if tmdb_api_key.is_empty() {
        println!("Skipping TMDB setup. Only anime search will be available.");
    } else {
        println!("TMDB API key saved. Movies and TV shows search enabled!");
    }

    // Save config
    let config = Config::new(rd_api_key, tmdb_api_key);
    save_config(&config)?;

    println!("\nConfiguration saved to: {}", config_path().display());
    println!("\nYou're all set! Run 'miru' to start watching.");

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
