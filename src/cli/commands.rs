use std::io::{self, Write};

use crate::api::{RealDebridClient, TmdbClient};
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
        println!();
    }

    // =========================================
    // Step 1: Prerequisites intro screen
    // =========================================
    println!("Before you start, make sure you have the following:\n");

    // Check if MPV is installed
    let mpv_installed = which::which("mpv").is_ok();
    if mpv_installed {
        println!("  [x] MPV media player (installed)");
    } else {
        println!("  [ ] MPV media player (NOT FOUND)");
        println!("      Install from: https://mpv.io/installation/");
    }

    println!();
    println!("  [x] TMDB API key (required, free)");
    println!("      Get yours at: https://www.themoviedb.org/settings/api");
    println!("      Use the \"API Key (v3 auth)\", not the Read Access Token.");
    println!();
    println!("  [ ] Real-Debrid API key (optional, paid subscription)");
    println!("      Sign up at: http://real-debrid.com/?id=16544328");
    println!("      Provides faster cached streaming. Without it, miru uses direct P2P.");
    println!();

    if !mpv_installed {
        println!("WARNING: MPV is not installed. You won't be able to play videos.");
        println!("         Please install MPV before continuing.\n");
    }

    print!("Press Enter to continue...");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    println!();

    // =========================================
    // Step 2: Real-Debrid API key (optional)
    // =========================================
    println!("Step 1/2: Real-Debrid (optional)\n");
    println!("Real-Debrid provides faster cached streaming for popular content.");
    println!("Without it, miru uses direct P2P streaming (free, but may buffer).");
    println!();
    println!("Get your API key at: https://real-debrid.com/apitoken\n");

    print!("Enter your Real-Debrid API key (or press Enter to skip): ");
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
        println!("Skipped. Using direct P2P streaming.");
    }
    println!();

    // =========================================
    // Step 3: TMDB API key (required)
    // =========================================
    println!("Step 2/2: TMDB (required)\n");
    println!("TMDB is required to search for movies, TV shows, and anime.");
    println!();
    println!("Get your API key at: https://www.themoviedb.org/settings/api");
    println!("Use the \"API Key (v3 auth)\", not the Read Access Token.\n");

    loop {
        print!("Enter your TMDB API key: ");
        io::stdout().flush()?;

        let mut tmdb_api_key = String::new();
        io::stdin().read_line(&mut tmdb_api_key)?;
        let tmdb_api_key = tmdb_api_key.trim().to_string();

        if tmdb_api_key.is_empty() {
            println!("API key cannot be empty. Please try again.\n");
            continue;
        }

        // Validate the TMDB API key
        print!("Validating TMDB API key... ");
        io::stdout().flush()?;

        let client = TmdbClient::new(tmdb_api_key.clone());
        match client.search_all("test").await {
            Ok(_) => {
                println!("OK!");

                // Save config
                let config = Config::new(rd_api_key, tmdb_api_key);
                save_config(&config)?;

                println!("\n=========================================");
                println!("Setup complete!");
                println!("=========================================\n");
                println!("Configuration saved to: {}", config_path().display());
                println!();
                println!("Run 'miru' to start watching.");

                return Ok(());
            }
            Err(e) => {
                println!("Failed!");
                println!("Error: {}", e);
                println!("Please check your API key and try again.\n");
            }
        }
    }
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
