use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::ui::components::Input;
use crate::ui::theme::Theme;

/// Search input screen
pub struct SearchScreen {
    pub input: Input,
}

impl SearchScreen {
    pub fn new() -> Self {
        Self {
            input: Input::new(),
        }
    }

    pub fn with_query(query: &str) -> Self {
        Self {
            input: Input::with_value(query.to_string()),
        }
    }

    /// Handle key input, returns Some(query) if search should be performed
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<String> {
        match key.code {
            KeyCode::Enter => {
                let query = self.input.get_value().trim().to_string();
                if !query.is_empty() {
                    return Some(query);
                }
            }
            KeyCode::Char(c) => {
                self.input.insert(c);
            }
            KeyCode::Backspace => {
                self.input.backspace();
            }
            KeyCode::Delete => {
                self.input.delete();
            }
            KeyCode::Left => {
                self.input.move_left();
            }
            KeyCode::Right => {
                self.input.move_right();
            }
            KeyCode::Home => {
                self.input.move_start();
            }
            KeyCode::End => {
                self.input.move_end();
            }
            _ => {}
        }
        None
    }

    /// Render the search screen
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(3), // Input
                Constraint::Length(2), // Help text
                Constraint::Min(0),    // Spacer
            ])
            .margin(2)
            .split(area);

        // Title
        let title = Line::from(vec![
            Span::styled("🎬 ", theme.normal()),
            Span::styled("miru", theme.title()),
        ]);
        let title_widget = Paragraph::new(title);
        frame.render_widget(title_widget, chunks[0]);

        // Search input
        self.input.render(frame, chunks[1], " Search ", theme);

        // Help text
        let help = Line::from(vec![
            Span::styled("Enter", theme.highlight()),
            Span::styled(" to search • ", theme.muted()),
            Span::styled("Esc", theme.highlight()),
            Span::styled(" to quit", theme.muted()),
        ]);
        let help_widget = Paragraph::new(help);
        frame.render_widget(help_widget, chunks[2]);
    }
}

impl Default for SearchScreen {
    fn default() -> Self {
        Self::new()
    }
}
