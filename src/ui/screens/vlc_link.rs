use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
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
                Constraint::Percentage(25),
                Constraint::Length(3), // Title
                Constraint::Length(2), // Spacing
                Constraint::Length(3), // Instructions
                Constraint::Length(2), // Spacing
                Constraint::Length(5), // VLC Link box
                Constraint::Length(2), // Spacing
                Constraint::Length(2), // Stream URL (truncated)
                Constraint::Length(3), // Spacing
                Constraint::Length(2), // Help text
                Constraint::Min(0),
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
            "Tap the link below to open in VLC for iOS",
            theme.normal(),
        )])])
        .alignment(Alignment::Center);
        frame.render_widget(instructions, chunks[3]);

        // VLC Link with OSC 8 hyperlink escape sequence
        // Format: \x1b]8;;URL\x1b\\LINK_TEXT\x1b]8;;\x1b\\
        let clickable_link = format!(
            "\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\",
            self.vlc_url, "[ Open in VLC ]"
        );

        let link_line = Line::from(vec![Span::styled(
            clickable_link,
            Style::default()
                .fg(ratatui::style::Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )]);
        let link_widget = Paragraph::new(link_line).alignment(Alignment::Center);
        frame.render_widget(link_widget, chunks[5]);

        // Stream URL (truncated for display)
        let display_url = if self.stream_url.len() > 60 {
            format!("{}...", &self.stream_url[..57])
        } else {
            self.stream_url.clone()
        };
        let url_info = Paragraph::new(Line::from(vec![
            Span::styled("Stream: ", theme.muted()),
            Span::styled(display_url, theme.muted()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(url_info, chunks[7]);

        // Help text
        let help = Paragraph::new(Line::from(vec![
            Span::styled("Press ", theme.muted()),
            Span::styled("Enter", theme.highlight()),
            Span::styled(" or ", theme.muted()),
            Span::styled("Esc", theme.highlight()),
            Span::styled(" to return", theme.muted()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(help, chunks[9]);
    }
}
