use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::config::{ThemeColors, UiConfig};

/// Theme variant for selecting color schemes
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeVariant {
    /// Terminal default colors - uses ANSI colors that adapt to terminal's theme
    Auto,
    /// Catppuccin Mocha - optimized for dark backgrounds
    Dark,
    /// Catppuccin Latte - optimized for light backgrounds
    Light,
}

impl ThemeVariant {
    /// Get the next variant in the cycle: Auto -> Dark -> Light -> Auto
    pub fn next(self) -> Self {
        match self {
            ThemeVariant::Auto => ThemeVariant::Dark,
            ThemeVariant::Dark => ThemeVariant::Light,
            ThemeVariant::Light => ThemeVariant::Auto,
        }
    }

    /// Convert to config string representation
    pub fn to_config_string(self) -> &'static str {
        match self {
            ThemeVariant::Auto => "auto",
            ThemeVariant::Dark => "dark",
            ThemeVariant::Light => "light",
        }
    }

    /// Parse from config string
    pub fn from_config_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "dark" => ThemeVariant::Dark,
            "light" => ThemeVariant::Light,
            _ => ThemeVariant::Auto,
        }
    }
}

impl Default for ThemeVariant {
    fn default() -> Self {
        ThemeVariant::Auto
    }
}

/// Catppuccin-inspired color theme with support for dark/light modes
/// and custom color overrides.
pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub muted: Color,
    pub text: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_config(&UiConfig::default())
    }
}

impl Theme {
    /// Create theme from UI config
    ///
    /// Theme selection:
    /// - "auto": Uses terminal default colors (ANSI colors that adapt automatically)
    /// - "dark": Uses Catppuccin Mocha (optimized for dark backgrounds)
    /// - "light": Uses Catppuccin Latte (optimized for light backgrounds)
    ///
    /// Custom color overrides from config are applied on top of the base theme.
    pub fn from_config(ui_config: &UiConfig) -> Self {
        let variant = ThemeVariant::from_config_string(&ui_config.theme);
        let mut theme = Self::from_variant(variant);

        // Apply any custom color overrides
        theme.apply_overrides(&ui_config.colors);
        theme
    }

    /// Create theme from variant
    pub fn from_variant(variant: ThemeVariant) -> Self {
        match variant {
            ThemeVariant::Auto => Self::terminal_default(),
            ThemeVariant::Dark => Self::catppuccin_mocha(),
            ThemeVariant::Light => Self::catppuccin_latte(),
        }
    }

    /// Terminal default theme - uses ANSI colors that adapt to terminal's theme
    ///
    /// This theme uses the terminal's default colors, which automatically adapt
    /// to the terminal's light/dark mode. The colors are mapped as follows:
    /// - text: Reset (terminal's default foreground)
    /// - primary: Blue
    /// - secondary: Magenta
    /// - success: Green
    /// - warning: Yellow
    /// - error: Red
    /// - muted: DarkGray
    pub fn terminal_default() -> Self {
        Self {
            primary: Color::Blue,
            secondary: Color::Magenta,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            muted: Color::DarkGray,
            text: Color::Reset, // Uses terminal's default foreground
        }
    }

    /// Catppuccin Mocha theme (dark background)
    /// https://github.com/catppuccin/catppuccin
    pub fn catppuccin_mocha() -> Self {
        Self {
            primary: Color::Rgb(137, 180, 250),   // blue #89b4fa
            secondary: Color::Rgb(245, 194, 231), // pink #f5c2e7
            success: Color::Rgb(166, 227, 161),   // green #a6e3a1
            warning: Color::Rgb(249, 226, 175),   // yellow #f9e2af
            error: Color::Rgb(243, 139, 168),     // red #f38ba8
            muted: Color::Rgb(108, 112, 134),     // overlay #6c7086
            text: Color::Rgb(205, 214, 244),      // text #cdd6f4
        }
    }

    /// Catppuccin Latte theme (light background)
    /// https://github.com/catppuccin/catppuccin
    pub fn catppuccin_latte() -> Self {
        Self {
            primary: Color::Rgb(30, 102, 245),    // blue #1e66f5
            secondary: Color::Rgb(234, 118, 203), // pink #ea76cb
            success: Color::Rgb(64, 160, 43),     // green #40a02b
            warning: Color::Rgb(223, 142, 29),    // yellow #df8e1d
            error: Color::Rgb(210, 15, 57),       // red #d20f39
            muted: Color::Rgb(108, 111, 133),     // overlay #6c6f85
            text: Color::Rgb(76, 79, 105),        // text #4c4f69
        }
    }

    /// Apply custom color overrides from config
    fn apply_overrides(&mut self, colors: &ThemeColors) {
        if let Some(ref color_str) = colors.primary {
            if let Some(color) = parse_hex_color(color_str) {
                self.primary = color;
            } else {
                warn!(
                    "Invalid color format for 'primary': '{}'. Expected '#RRGGBB' format.",
                    color_str
                );
            }
        }
        if let Some(ref color_str) = colors.secondary {
            if let Some(color) = parse_hex_color(color_str) {
                self.secondary = color;
            } else {
                warn!(
                    "Invalid color format for 'secondary': '{}'. Expected '#RRGGBB' format.",
                    color_str
                );
            }
        }
        if let Some(ref color_str) = colors.success {
            if let Some(color) = parse_hex_color(color_str) {
                self.success = color;
            } else {
                warn!(
                    "Invalid color format for 'success': '{}'. Expected '#RRGGBB' format.",
                    color_str
                );
            }
        }
        if let Some(ref color_str) = colors.warning {
            if let Some(color) = parse_hex_color(color_str) {
                self.warning = color;
            } else {
                warn!(
                    "Invalid color format for 'warning': '{}'. Expected '#RRGGBB' format.",
                    color_str
                );
            }
        }
        if let Some(ref color_str) = colors.error {
            if let Some(color) = parse_hex_color(color_str) {
                self.error = color;
            } else {
                warn!(
                    "Invalid color format for 'error': '{}'. Expected '#RRGGBB' format.",
                    color_str
                );
            }
        }
        if let Some(ref color_str) = colors.muted {
            if let Some(color) = parse_hex_color(color_str) {
                self.muted = color;
            } else {
                warn!(
                    "Invalid color format for 'muted': '{}'. Expected '#RRGGBB' format.",
                    color_str
                );
            }
        }
        if let Some(ref color_str) = colors.text {
            if let Some(color) = parse_hex_color(color_str) {
                self.text = color;
            } else {
                warn!(
                    "Invalid color format for 'text': '{}'. Expected '#RRGGBB' format.",
                    color_str
                );
            }
        }
    }

    /// Style for normal text
    pub fn normal(&self) -> Style {
        Style::default().fg(self.text)
    }

    /// Style for highlighted/selected items
    pub fn highlight(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for muted/secondary text
    pub fn muted(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// Style for warning messages
    pub fn warning(&self) -> Style {
        Style::default().fg(self.warning)
    }

    /// Style for error messages
    pub fn error(&self) -> Style {
        Style::default().fg(self.error)
    }

    /// Style for the title/header
    pub fn title(&self) -> Style {
        Style::default()
            .fg(self.secondary)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for borders
    pub fn border(&self) -> Style {
        Style::default().fg(self.muted)
    }

    /// Style for selected list item
    pub fn selected(&self) -> Style {
        Style::default()
            .fg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

    /// Style for accent (movies)
    pub fn accent(&self) -> Style {
        Style::default().fg(self.secondary)
    }

    /// Style for info (TV shows)
    pub fn info(&self) -> Style {
        Style::default().fg(self.success)
    }

    /// Style for success indicators (checkmarks, etc.)
    pub fn success(&self) -> Style {
        Style::default().fg(self.success)
    }
}

/// Parse a hex color string in "#RRGGBB" format
///
/// Returns None if the format is invalid, allowing the caller to
/// fall back to the default color and log a warning.
fn parse_hex_color(s: &str) -> Option<Color> {
    let s = s.trim();

    // Must start with # and be exactly 7 characters
    if !s.starts_with('#') || s.len() != 7 {
        return None;
    }

    let r = u8::from_str_radix(&s[1..3], 16).ok()?;
    let g = u8::from_str_radix(&s[3..5], 16).ok()?;
    let b = u8::from_str_radix(&s[5..7], 16).ok()?;

    Some(Color::Rgb(r, g, b))
}

/// Selection arrow character
pub const ARROW: &str = "❯";

/// Spinner frames for loading animation
pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Star character for ratings
pub const STAR: &str = "★";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color_valid() {
        assert_eq!(parse_hex_color("#000000"), Some(Color::Rgb(0, 0, 0)));
        assert_eq!(parse_hex_color("#ffffff"), Some(Color::Rgb(255, 255, 255)));
        assert_eq!(parse_hex_color("#FFFFFF"), Some(Color::Rgb(255, 255, 255)));
        assert_eq!(parse_hex_color("#89b4fa"), Some(Color::Rgb(137, 180, 250)));
        assert_eq!(parse_hex_color("#ff6600"), Some(Color::Rgb(255, 102, 0)));
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        assert_eq!(parse_hex_color(""), None);
        assert_eq!(parse_hex_color("000000"), None); // Missing #
        assert_eq!(parse_hex_color("#fff"), None); // Too short
        assert_eq!(parse_hex_color("#ffffffff"), None); // Too long
        assert_eq!(parse_hex_color("#gggggg"), None); // Invalid hex
        assert_eq!(parse_hex_color("red"), None); // Named color not supported
    }

    #[test]
    fn test_terminal_default_theme() {
        let theme = Theme::terminal_default();
        assert_eq!(theme.text, Color::Reset);
        assert_eq!(theme.primary, Color::Blue);
        assert_eq!(theme.secondary, Color::Magenta);
        assert_eq!(theme.success, Color::Green);
        assert_eq!(theme.warning, Color::Yellow);
        assert_eq!(theme.error, Color::Red);
        assert_eq!(theme.muted, Color::DarkGray);
    }

    #[test]
    fn test_catppuccin_mocha_colors() {
        let theme = Theme::catppuccin_mocha();
        assert_eq!(theme.primary, Color::Rgb(137, 180, 250));
        assert_eq!(theme.text, Color::Rgb(205, 214, 244));
    }

    #[test]
    fn test_catppuccin_latte_colors() {
        let theme = Theme::catppuccin_latte();
        assert_eq!(theme.primary, Color::Rgb(30, 102, 245));
        assert_eq!(theme.text, Color::Rgb(76, 79, 105));
    }

    #[test]
    fn test_theme_from_variant() {
        assert_eq!(Theme::from_variant(ThemeVariant::Auto).text, Color::Reset);
        assert_eq!(
            Theme::from_variant(ThemeVariant::Dark).primary,
            Color::Rgb(137, 180, 250)
        );
        assert_eq!(
            Theme::from_variant(ThemeVariant::Light).primary,
            Color::Rgb(30, 102, 245)
        );
    }

    #[test]
    fn test_theme_variant_cycle() {
        assert_eq!(ThemeVariant::Auto.next(), ThemeVariant::Dark);
        assert_eq!(ThemeVariant::Dark.next(), ThemeVariant::Light);
        assert_eq!(ThemeVariant::Light.next(), ThemeVariant::Auto);
    }

    #[test]
    fn test_theme_variant_from_config_string() {
        assert_eq!(ThemeVariant::from_config_string("auto"), ThemeVariant::Auto);
        assert_eq!(ThemeVariant::from_config_string("dark"), ThemeVariant::Dark);
        assert_eq!(
            ThemeVariant::from_config_string("light"),
            ThemeVariant::Light
        );
        assert_eq!(ThemeVariant::from_config_string("DARK"), ThemeVariant::Dark);
        assert_eq!(
            ThemeVariant::from_config_string("unknown"),
            ThemeVariant::Auto
        );
    }

    #[test]
    fn test_theme_from_config_dark() {
        let config = UiConfig {
            theme: "dark".to_string(),
            colors: ThemeColors::default(),
        };
        let theme = Theme::from_config(&config);
        assert_eq!(theme.primary, Color::Rgb(137, 180, 250)); // Mocha blue
    }

    #[test]
    fn test_theme_from_config_light() {
        let config = UiConfig {
            theme: "light".to_string(),
            colors: ThemeColors::default(),
        };
        let theme = Theme::from_config(&config);
        assert_eq!(theme.primary, Color::Rgb(30, 102, 245)); // Latte blue
    }

    #[test]
    fn test_theme_from_config_auto() {
        let config = UiConfig {
            theme: "auto".to_string(),
            colors: ThemeColors::default(),
        };
        let theme = Theme::from_config(&config);
        assert_eq!(theme.text, Color::Reset); // Terminal default
    }

    #[test]
    fn test_theme_custom_override() {
        let config = UiConfig {
            theme: "dark".to_string(),
            colors: ThemeColors {
                primary: Some("#ff6600".to_string()),
                text: Some("#ffffff".to_string()),
                ..Default::default()
            },
        };
        let theme = Theme::from_config(&config);
        assert_eq!(theme.primary, Color::Rgb(255, 102, 0)); // Custom orange
        assert_eq!(theme.text, Color::Rgb(255, 255, 255)); // Custom white
        assert_eq!(theme.secondary, Color::Rgb(245, 194, 231)); // Default Mocha pink
    }

    #[test]
    fn test_theme_invalid_color_uses_default() {
        let config = UiConfig {
            theme: "dark".to_string(),
            colors: ThemeColors {
                primary: Some("invalid".to_string()),
                ..Default::default()
            },
        };
        let theme = Theme::from_config(&config);
        // Should fall back to default Mocha primary
        assert_eq!(theme.primary, Color::Rgb(137, 180, 250));
    }
}
