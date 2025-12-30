use serde::{Deserialize, Serialize};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub real_debrid: RealDebridConfig,

    #[serde(default)]
    pub tmdb: TmdbConfig,

    #[serde(default)]
    pub torrentio: TorrentioConfig,

    #[serde(default)]
    pub player: PlayerConfig,

    #[serde(default)]
    pub ui: UiConfig,
}

impl Config {
    /// Create a new config with just the API keys, using defaults for everything else
    pub fn new(rd_api_key: String, tmdb_api_key: String) -> Self {
        Self {
            real_debrid: RealDebridConfig { api_key: rd_api_key },
            tmdb: TmdbConfig { api_key: tmdb_api_key },
            torrentio: TorrentioConfig::default(),
            player: PlayerConfig::default(),
            ui: UiConfig::default(),
        }
    }

    /// Check if the config has a valid API key
    pub fn has_api_key(&self) -> bool {
        !self.real_debrid.api_key.is_empty()
    }
}

/// Real-Debrid configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealDebridConfig {
    pub api_key: String,
}

/// TMDB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbConfig {
    pub api_key: String,
}

impl Default for TmdbConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
        }
    }
}

/// Torrentio addon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentioConfig {
    /// Providers ordered by priority
    #[serde(default = "default_providers")]
    pub providers: Vec<String>,

    /// Quality preference: "best" | "1080p" | "720p" | "480p"
    #[serde(default = "default_quality")]
    pub quality: String,

    /// Sort strategy: "quality" | "size" | "seeders"
    #[serde(default = "default_sort")]
    pub sort: String,
}

impl Default for TorrentioConfig {
    fn default() -> Self {
        Self {
            providers: default_providers(),
            quality: default_quality(),
            sort: default_sort(),
        }
    }
}

fn default_providers() -> Vec<String> {
    vec![
        "yts".to_string(),
        "eztv".to_string(),
        "rarbg".to_string(),
        "1337x".to_string(),
        "thepiratebay".to_string(),
        "kickasstorrents".to_string(),
        "torrentgalaxy".to_string(),
        "nyaasi".to_string(),
    ]
}

fn default_quality() -> String {
    "best".to_string()
}

fn default_sort() -> String {
    "quality".to_string()
}

/// Player configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerConfig {
    /// Command to launch player
    #[serde(default = "default_player_command")]
    pub command: String,

    /// Additional arguments passed to player
    #[serde(default)]
    pub args: Vec<String>,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            command: default_player_command(),
            args: vec!["--fullscreen".to_string()],
        }
    }
}

fn default_player_command() -> String {
    "mpv".to_string()
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Color theme: "default" | "minimal" | "dracula" | "catppuccin"
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
        }
    }
}

fn default_theme() -> String {
    "default".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new() {
        let config = Config::new("test_key".to_string(), "tmdb_key".to_string());
        assert_eq!(config.real_debrid.api_key, "test_key");
        assert_eq!(config.tmdb.api_key, "tmdb_key");
        assert!(config.has_api_key());
    }

    #[test]
    fn test_config_empty_key() {
        let config = Config::new("".to_string(), "".to_string());
        assert!(!config.has_api_key());
    }

    #[test]
    fn test_default_providers() {
        let config = TorrentioConfig::default();
        assert!(!config.providers.is_empty());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::new("my_api_key".to_string(), "my_tmdb_key".to_string());
        let toml_str = toml::to_string(&config).unwrap();
        assert!(toml_str.contains("my_api_key"));
        assert!(toml_str.contains("my_tmdb_key"));
    }

    #[test]
    fn test_config_deserialization_minimal() {
        let toml_str = r#"
[real_debrid]
api_key = "test_key"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.real_debrid.api_key, "test_key");
        assert_eq!(config.player.command, "mpv");
        assert_eq!(config.torrentio.quality, "best");
    }

    #[test]
    fn test_config_deserialization_full() {
        let toml_str = r#"
[real_debrid]
api_key = "test_key"

[torrentio]
providers = ["nyaasi"]
quality = "1080p"
sort = "seeders"

[player]
command = "vlc"
args = ["--fullscreen", "--loop"]

[ui]
theme = "dracula"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.real_debrid.api_key, "test_key");
        assert_eq!(config.torrentio.providers, vec!["nyaasi"]);
        assert_eq!(config.torrentio.quality, "1080p");
        assert_eq!(config.player.command, "vlc");
        assert_eq!(config.ui.theme, "dracula");
    }
}
