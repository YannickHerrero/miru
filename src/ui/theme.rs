use ratatui::style::{Color, Modifier, Style};

/// Catppuccin-inspired color theme
pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub muted: Color,
    pub text: Color,
    #[allow(dead_code)]
    pub background: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self::catppuccin()
    }
}

impl Theme {
    /// Catppuccin Mocha inspired theme
    pub fn catppuccin() -> Self {
        Self {
            primary: Color::Rgb(137, 180, 250),    // blue #89b4fa
            secondary: Color::Rgb(245, 194, 231),  // pink #f5c2e7
            success: Color::Rgb(166, 227, 161),    // green #a6e3a1
            warning: Color::Rgb(249, 226, 175),    // yellow #f9e2af
            error: Color::Rgb(243, 139, 168),      // red #f38ba8
            muted: Color::Rgb(108, 112, 134),      // overlay #6c7086
            text: Color::Rgb(205, 214, 244),       // text #cdd6f4
            background: Color::Reset,             // terminal default
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

    /// Style for success messages
    #[allow(dead_code)]
    pub fn success(&self) -> Style {
        Style::default().fg(self.success)
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
}

/// Selection arrow character
pub const ARROW: &str = "❯";

/// Spinner frames for loading animation
pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Cache status indicators
#[allow(dead_code)]
pub const CACHED_INDICATOR: &str = "🟢";
#[allow(dead_code)]
pub const NOT_CACHED_INDICATOR: &str = "🟡";

/// Star character for ratings
pub const STAR: &str = "★";
