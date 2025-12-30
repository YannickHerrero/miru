use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::theme::Theme;

/// Action from error screen
pub enum ErrorAction {
    Retry,
    Back,
}

/// Error display screen
pub struct ErrorScreen {
    pub message: String,
    pub can_retry: bool,
}

impl ErrorScreen {
    pub fn new(message: impl Into<String>, can_retry: bool) -> Self {
        Self {
            message: message.into(),
            can_retry,
        }
    }

    /// Handle key input
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<ErrorAction> {
        match key.code {
            KeyCode::Char('r') if self.can_retry => {
                return Some(ErrorAction::Retry);
            }
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                return Some(ErrorAction::Back);
            }
            _ => {}
        }
        None
    }

    /// Render the error screen
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Length(3),
                Constraint::Length(2),
                Constraint::Min(0),
            ])
            .split(area);

        // Error message
        let error_line = Line::from(vec![
            Span::styled("Error: ", theme.error()),
            Span::styled(&self.message, theme.normal()),
        ]);
        let error_widget = Paragraph::new(error_line).alignment(Alignment::Center);
        frame.render_widget(error_widget, chunks[1]);

        // Help text
        let mut help_spans = vec![];
        
        if self.can_retry {
            help_spans.extend([
                Span::styled("r", theme.highlight()),
                Span::styled(" retry • ", theme.muted()),
            ]);
        }
        
        help_spans.extend([
            Span::styled("Enter/Esc", theme.highlight()),
            Span::styled(" go back", theme.muted()),
        ]);

        let help = Line::from(help_spans);
        let help_widget = Paragraph::new(help).alignment(Alignment::Center);
        frame.render_widget(help_widget, chunks[2]);
    }
}
