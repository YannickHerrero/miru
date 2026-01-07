use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::ui::theme::Theme;

/// Action from VLC link screen
pub enum VlcLinkAction {
    /// User pressed a key to return to the TUI
    Back,
}

/// Screen displaying a clickable VLC URL for iOS users
pub struct VlcLinkScreen {
    /// The VLC URL scheme link (vlc://...)
    vlc_url: String,
    /// The original stream URL (for display purposes)
    #[allow(dead_code)]
    stream_url: String,
    /// Optional media title for display
    title: Option<String>,
}

impl VlcLinkScreen {
    pub fn new(vlc_url: String, stream_url: String, title: Option<String>) -> Self {
        Self {
            vlc_url,
            stream_url,
            title,
        }
    }

    /// Handle key input - any key returns to the TUI
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<VlcLinkAction> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') | KeyCode::Char(' ') => {
                Some(VlcLinkAction::Back)
            }
            _ => None,
        }
    }

    /// Render the VLC link screen
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Top padding
                Constraint::Length(2), // Title
                Constraint::Length(1), // Spacing
                Constraint::Length(2), // Instructions
                Constraint::Length(1), // Spacing
                Constraint::Min(6),    // VLC URL (flexible height for wrapping)
                Constraint::Length(1), // Spacing
                Constraint::Length(2), // Help text
                Constraint::Length(1), // Bottom padding
            ])
            .split(area);

        // Title
        let title_text = if let Some(ref title) = self.title {
            format!("Open in VLC: {}", title)
        } else {
            "Open in VLC".to_string()
        };
        let title = Paragraph::new(Line::from(vec![Span::styled(
            title_text,
            theme.highlight().add_modifier(Modifier::BOLD),
        )]))
        .alignment(Alignment::Center);
        frame.render_widget(title, chunks[1]);

        // Instructions
        let instructions = Paragraph::new(vec![Line::from(vec![Span::styled(
            "Tap the link below to open in VLC:",
            theme.normal(),
        )])])
        .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[3]);

        // VLC URL - displayed as plain text without OSC 8
        // iOS terminal should auto-detect it as a clickable URL
        let url_widget = Paragraph::new(Line::from(vec![Span::styled(
            self.vlc_url.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::UNDERLINED),
        )]))
        .wrap(Wrap { trim: false })
        .alignment(Alignment::Center);
        frame.render_widget(url_widget, chunks[5]);

        // Help text
        let help = Paragraph::new(Line::from(vec![
            Span::styled("Press ", theme.muted()),
            Span::styled("Enter", theme.highlight()),
            Span::styled(" or ", theme.muted()),
            Span::styled("Esc", theme.highlight()),
            Span::styled(" to return", theme.muted()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(help, chunks[7]);
    }
}
