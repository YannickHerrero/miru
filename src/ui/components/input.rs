use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::ui::theme::Theme;

/// Text input component
pub struct Input {
    /// Current input value
    pub value: String,
    /// Cursor position
    pub cursor: usize,
    /// Whether the input is focused
    pub focused: bool,
}

impl Input {
    pub fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
            focused: true,
        }
    }

    pub fn with_value(value: String) -> Self {
        let cursor = value.len();
        Self {
            value,
            cursor,
            focused: true,
        }
    }

    /// Handle character input
    pub fn insert(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += 1;
    }

    /// Handle backspace
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.value.remove(self.cursor);
        }
    }

    /// Handle delete
    pub fn delete(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    /// Move cursor left
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Move cursor right
    pub fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += 1;
        }
    }

    /// Move cursor to start
    pub fn move_start(&mut self) {
        self.cursor = 0;
    }

    /// Move cursor to end
    pub fn move_end(&mut self) {
        self.cursor = self.value.len();
    }

    /// Get the current value
    pub fn get_value(&self) -> &str {
        &self.value
    }

    /// Render the input
    pub fn render(&self, frame: &mut Frame, area: Rect, title: &str, theme: &Theme) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(if self.focused {
                theme.highlight()
            } else {
                theme.border()
            })
            .title(title);

        // Build the input line with cursor
        let (before, after) = self.value.split_at(self.cursor);
        let cursor_char = after.chars().next().unwrap_or(' ');
        let after = if after.is_empty() { "" } else { &after[cursor_char.len_utf8()..] };

        let line = if self.focused {
            Line::from(vec![
                Span::styled(before, theme.normal()),
                Span::styled(
                    cursor_char.to_string(),
                    Style::default().add_modifier(Modifier::REVERSED),
                ),
                Span::styled(after, theme.normal()),
            ])
        } else {
            Line::from(Span::styled(&self.value, theme.normal()))
        };

        let paragraph = Paragraph::new(line).block(block);
        frame.render_widget(paragraph, area);
    }
}

impl Default for Input {
    fn default() -> Self {
        Self::new()
    }
}
