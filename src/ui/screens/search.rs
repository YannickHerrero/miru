use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::history::WatchedItem;
use crate::ui::components::Input;
use crate::ui::theme::Theme;

/// Action from search screen
pub enum SearchAction {
    /// Perform search with query
    Search(String),
    /// Select item from history
    SelectHistory(WatchedItem),
}

/// Focus state for the search screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Search,
    History,
}

/// Search input screen with watch history
pub struct SearchScreen {
    pub input: Input,
    /// Recent watch history
    history: Vec<WatchedItem>,
    /// Currently selected history item
    history_selected: usize,
    /// History list state
    history_state: ListState,
    /// Current focus (search bar or history)
    focus: Focus,
}

impl SearchScreen {
    pub fn new() -> Self {
        Self {
            input: Input::new(),
            history: Vec::new(),
            history_selected: 0,
            history_state: ListState::default(),
            focus: Focus::Search,
        }
    }

    pub fn with_query(query: &str) -> Self {
        Self {
            input: Input::with_value(query.to_string()),
            history: Vec::new(),
            history_selected: 0,
            history_state: ListState::default(),
            focus: Focus::Search,
        }
    }

    pub fn with_query_and_history(query: &str, history: Vec<WatchedItem>) -> Self {
        let mut state = ListState::default();
        if !history.is_empty() {
            state.select(Some(0));
        }
        Self {
            input: Input::with_value(query.to_string()),
            history,
            history_selected: 0,
            history_state: state,
            focus: Focus::Search,
        }
    }

    pub fn new_with_history(history: Vec<WatchedItem>) -> Self {
        let mut state = ListState::default();
        if !history.is_empty() {
            state.select(Some(0));
        }
        Self {
            input: Input::new(),
            history,
            history_selected: 0,
            history_state: state,
            focus: Focus::Search,
        }
    }

    /// Set the watch history
    #[allow(dead_code)]
    pub fn set_history(&mut self, history: Vec<WatchedItem>) {
        self.history = history;
        self.history_selected = 0;
        if !self.history.is_empty() {
            self.history_state.select(Some(0));
        } else {
            self.history_state.select(None);
        }
    }

    /// Handle key input, returns Some(action) if an action should be performed
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<SearchAction> {
        match self.focus {
            Focus::Search => self.handle_search_key(key),
            Focus::History => self.handle_history_key(key),
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Option<SearchAction> {
        match key.code {
            KeyCode::Enter => {
                let query = self.input.get_value().trim().to_string();
                if !query.is_empty() {
                    return Some(SearchAction::Search(query));
                }
            }
            KeyCode::Down | KeyCode::Tab => {
                // Move focus to history if available
                if !self.history.is_empty() {
                    self.focus = Focus::History;
                    self.history_state.select(Some(self.history_selected));
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

    fn handle_history_key(&mut self, key: KeyEvent) -> Option<SearchAction> {
        match key.code {
            KeyCode::Enter => {
                if let Some(item) = self.history.get(self.history_selected) {
                    return Some(SearchAction::SelectHistory(item.clone()));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.history_selected == 0 {
                    // Move back to search
                    self.focus = Focus::Search;
                    self.history_state.select(None);
                } else {
                    self.history_selected = self.history_selected.saturating_sub(1);
                    self.history_state.select(Some(self.history_selected));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.history_selected < self.history.len().saturating_sub(1) {
                    self.history_selected += 1;
                    self.history_state.select(Some(self.history_selected));
                }
            }
            KeyCode::Tab | KeyCode::Esc => {
                // Move back to search
                self.focus = Focus::Search;
                self.history_state.select(None);
            }
            KeyCode::Char(c) => {
                // Start typing - switch to search and insert character
                self.focus = Focus::Search;
                self.history_state.select(None);
                self.input.insert(c);
            }
            KeyCode::Backspace => {
                // Switch to search and handle backspace
                self.focus = Focus::Search;
                self.history_state.select(None);
                self.input.backspace();
            }
            _ => {}
        }
        None
    }

    /// Render the search screen
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let has_history = !self.history.is_empty();

        let constraints = if has_history {
            vec![
                Constraint::Length(3), // Title
                Constraint::Length(3), // Input
                Constraint::Length(2), // Help text
                Constraint::Length(1), // Spacer
                Constraint::Min(5),    // History
            ]
        } else {
            vec![
                Constraint::Length(3), // Title
                Constraint::Length(3), // Input
                Constraint::Length(2), // Help text
                Constraint::Min(0),    // Spacer
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .margin(2)
            .split(area);

        // Title
        let title = Line::from(vec![Span::styled("miru", theme.title())]);
        let title_widget = Paragraph::new(title);
        frame.render_widget(title_widget, chunks[0]);

        // Search input with focus indicator
        let input_style = if self.focus == Focus::Search {
            theme.highlight()
        } else {
            theme.border()
        };
        self.input
            .render_with_style(frame, chunks[1], " Search ", theme, input_style);

        // Help text
        let help = if has_history {
            Line::from(vec![
                Span::styled("Enter", theme.highlight()),
                Span::styled(" to search ", theme.muted()),
                Span::styled("Tab/Down", theme.highlight()),
                Span::styled(" history ", theme.muted()),
                Span::styled("Esc", theme.highlight()),
                Span::styled(" quit", theme.muted()),
            ])
        } else {
            Line::from(vec![
                Span::styled("Enter", theme.highlight()),
                Span::styled(" to search ", theme.muted()),
                Span::styled("Esc", theme.highlight()),
                Span::styled(" to quit", theme.muted()),
            ])
        };
        let help_widget = Paragraph::new(help);
        frame.render_widget(help_widget, chunks[2]);

        // History section
        if has_history {
            self.render_history(frame, chunks[4], theme);
        }
    }

    fn render_history(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self
            .history
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_selected = self.focus == Focus::History && i == self.history_selected;

                let mut spans = vec![];

                // Selection indicator
                if is_selected {
                    spans.push(Span::styled("> ", theme.selected()));
                } else {
                    spans.push(Span::raw("  "));
                }

                // Title
                let title_style = if is_selected {
                    theme.selected()
                } else {
                    theme.normal()
                };
                spans.push(Span::styled(&item.title, title_style));

                // Episode info for TV shows
                let ep_display = item.episode_display();
                if !ep_display.is_empty() {
                    spans.push(Span::styled(format!(" {}", ep_display), theme.muted()));
                }

                // Media type badge
                let type_badge = match item.media_type {
                    crate::api::MediaType::Movie => " [Movie]",
                    crate::api::MediaType::TvShow => " [TV]",
                };
                spans.push(Span::styled(type_badge, theme.muted()));

                // Time ago
                spans.push(Span::styled(
                    format!(" - {}", item.watched_at_display()),
                    theme.muted(),
                ));

                ListItem::new(Line::from(spans))
            })
            .collect();

        let border_style = if self.focus == Focus::History {
            theme.highlight()
        } else {
            theme.border()
        };

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Recently Watched "),
        );

        frame.render_stateful_widget(list, area, &mut self.history_state);
    }
}

impl Default for SearchScreen {
    fn default() -> Self {
        Self::new()
    }
}
