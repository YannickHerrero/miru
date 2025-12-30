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
    println!("Welcome to miru - your terminal anime streaming companion!\n");

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

    // Prompt for API key
    println!("To use miru, you need a Real-Debrid API key.");
    println!("Get yours at: https://real-debrid.com/apitoken\n");

    print!("Enter your Real-Debrid API key: ");
    io::stdout().flush()?;

    let mut api_key = String::new();
    io::stdin().read_line(&mut api_key)?;
    let api_key = api_key.trim().to_string();

    if api_key.is_empty() {
        println!("API key cannot be empty. Setup cancelled.");
        return Ok(());
    }

    // Validate the API key
    print!("Validating API key... ");
    io::stdout().flush()?;

    let client = RealDebridClient::new(api_key.clone());
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

    // Save config
    let config = Config::new(api_key);
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
            println!("Available keys: rd_api_key");
            return Ok(());
        }

        let mut config = load_config().unwrap_or_else(|_| Config::new(String::new()));

        match parts[0] {
            "rd_api_key" => {
                config.real_debrid.api_key = parts[1].to_string();
            }
            _ => {
                println!("Unknown key: {}", parts[0]);
                println!("Available keys: rd_api_key");
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
