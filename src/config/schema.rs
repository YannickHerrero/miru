use serde::{Deserialize, Serialize};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Real-Debrid configuration (optional - if not set, direct P2P streaming is used)
    #[serde(default)]
    pub real_debrid: RealDebridConfig,

    #[serde(default)]
    pub tmdb: TmdbConfig,

    #[serde(default)]
    pub torrentio: TorrentioConfig,

    #[serde(default)]
    pub player: PlayerConfig,

    #[serde(default)]
    pub ui: UiConfig,

    /// Direct P2P streaming configuration (used when Real-Debrid is not configured)
    #[serde(default)]
    pub streaming: StreamingConfig,
}

impl Config {
    /// Create a new config with just the API keys, using defaults for everything else
    pub fn new(rd_api_key: String, tmdb_api_key: String) -> Self {
        Self {
            real_debrid: RealDebridConfig {
                api_key: rd_api_key,
            },
            tmdb: TmdbConfig {
                api_key: tmdb_api_key,
            },
            torrentio: TorrentioConfig::default(),
            player: PlayerConfig::default(),
            ui: UiConfig::default(),
            streaming: StreamingConfig::default(),
        }
    }

    /// Check if the config has a valid Real-Debrid API key
    pub fn has_rd_api_key(&self) -> bool {
        !self.real_debrid.api_key.is_empty()
    }

    /// Check if direct P2P streaming should be used (no RD key configured)
    pub fn use_direct_streaming(&self) -> bool {
        !self.has_rd_api_key()
    }
}

/// Real-Debrid configuration (optional)
///
/// Real-Debrid is optional. If no API key is configured, miru will use direct P2P streaming.
///
/// - **With Real-Debrid**: Access cached torrents on Real-Debrid servers for faster speeds
/// - **Without Real-Debrid**: Use direct P2P streaming to download torrents to your device
///
/// To add a Real-Debrid account later, run: `miru config --set rd_api_key=YOUR_KEY`
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RealDebridConfig {
    /// Real-Debrid API key (optional). Leave empty to use direct P2P streaming.
    /// Get yours at: https://real-debrid.com/apitoken
    #[serde(default)]
    pub api_key: String,
}

/// TMDB configuration (required)
///
/// TMDB (The Movie Database) is required for all search functionality.
/// Without a valid TMDB API key, search will return no results.
///
/// Get your API key at: https://www.themoviedb.org/settings/api
/// Use the "API Key (v3 auth)", not the Read Access Token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TmdbConfig {
    /// TMDB API key (required). Must be obtained from https://www.themoviedb.org/settings/api
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

impl PlayerConfig {
    /// Create a VLC player configuration with sensible defaults
    pub fn vlc() -> Self {
        Self {
            command: "vlc".to_string(),
            args: vec!["--fullscreen".to_string(), "--play-and-exit".to_string()],
        }
    }
}

fn default_player_command() -> String {
    "mpv".to_string()
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Theme mode: "auto", "dark", or "light"
    /// - "auto": Uses terminal's default ANSI colors (automatically adapts to light/dark)
    /// - "dark": Use Catppuccin Mocha (optimized for dark backgrounds)
    /// - "light": Use Catppuccin Latte (optimized for light backgrounds)
    ///
    /// Press Ctrl+T at any time to cycle through themes.
    #[serde(default = "default_theme_mode")]
    pub theme: String,

    /// Custom color overrides (optional)
    /// These colors override the base theme colors.
    /// Format: "#RRGGBB" hex colors (e.g., "#89b4fa")
    #[serde(default)]
    pub colors: ThemeColors,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            theme: default_theme_mode(),
            colors: ThemeColors::default(),
        }
    }
}

fn default_theme_mode() -> String {
    "auto".to_string()
}

/// Custom theme colors (all optional - uses base theme defaults if not specified)
///
/// All colors should be specified as hex strings in "#RRGGBB" format.
///
/// Example in config.toml:
/// ```text
/// [ui]
/// theme = "dark"
///
/// [ui.colors]
/// primary = "#ff6600"
/// text = "#ffffff"
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThemeColors {
    /// Primary color (highlights, selected items, keybindings)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<String>,

    /// Secondary color (titles, movie badges)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary: Option<String>,

    /// Success color (TV badges, checkmarks)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub success: Option<String>,

    /// Warning color (HDR labels, ratings)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,

    /// Error color (errors, uncached indicators)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Muted color (secondary text, borders)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub muted: Option<String>,

    /// Text color (normal text)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Direct P2P streaming configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// HTTP port for the local streaming server (default: 3131)
    #[serde(default = "default_streaming_port")]
    pub http_port: u16,

    /// Whether to delete downloaded files after playback (default: true)
    #[serde(default = "default_cleanup_after_playback")]
    pub cleanup_after_playback: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            http_port: default_streaming_port(),
            cleanup_after_playback: default_cleanup_after_playback(),
        }
    }
}

fn default_streaming_port() -> u16 {
    3131
}

fn default_cleanup_after_playback() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new() {
        let config = Config::new("test_key".to_string(), "tmdb_key".to_string());
        assert_eq!(config.real_debrid.api_key, "test_key");
        assert_eq!(config.tmdb.api_key, "tmdb_key");
        assert!(config.has_rd_api_key());
        assert!(!config.use_direct_streaming());
    }

    #[test]
    fn test_config_empty_key() {
        let config = Config::new("".to_string(), "".to_string());
        assert!(!config.has_rd_api_key());
        assert!(config.use_direct_streaming());
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
theme = "dark"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.real_debrid.api_key, "test_key");
        assert_eq!(config.torrentio.providers, vec!["nyaasi"]);
        assert_eq!(config.torrentio.quality, "1080p");
        assert_eq!(config.player.command, "vlc");
        assert_eq!(config.ui.theme, "dark");
    }

    #[test]
    fn test_config_ui_custom_colors() {
        let toml_str = r##"
[ui]
theme = "dark"

[ui.colors]
primary = "#ff6600"
text = "#ffffff"
"##;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.ui.theme, "dark");
        assert_eq!(config.ui.colors.primary, Some("#ff6600".to_string()));
        assert_eq!(config.ui.colors.text, Some("#ffffff".to_string()));
        assert_eq!(config.ui.colors.secondary, None);
    }

    #[test]
    fn test_config_ui_auto_theme() {
        let config = UiConfig::default();
        assert_eq!(config.theme, "auto");
        assert!(config.colors.primary.is_none());
    }
}
