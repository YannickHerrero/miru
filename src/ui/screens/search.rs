use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::history::{WatchedItem, WatchlistItem};
use crate::ui::components::Input;
use crate::ui::theme::Theme;

/// Action from search screen
pub enum SearchAction {
    /// Perform search with query
    Search(String),
    /// Select item from history
    SelectHistory(WatchedItem),
    /// Select item from watchlist
    SelectWatchlist(WatchlistItem),
    /// Remove item from watchlist
    RemoveFromWatchlist(WatchlistItem),
}

/// Focus state for the search screen
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Search,
    History,
    Watchlist,
}

/// Search input screen with watch history and watchlist
pub struct SearchScreen {
    pub input: Input,
    /// Recent watch history
    history: Vec<WatchedItem>,
    /// Currently selected history item
    history_selected: usize,
    /// History list state
    history_state: ListState,
    /// Watchlist items
    watchlist: Vec<WatchlistItem>,
    /// Currently selected watchlist item
    watchlist_selected: usize,
    /// Watchlist list state
    watchlist_state: ListState,
    /// Current focus (search bar, history, or watchlist)
    focus: Focus,
}

impl SearchScreen {
    pub fn new() -> Self {
        Self {
            input: Input::new(),
            history: Vec::new(),
            history_selected: 0,
            history_state: ListState::default(),
            watchlist: Vec::new(),
            watchlist_selected: 0,
            watchlist_state: ListState::default(),
            focus: Focus::Search,
        }
    }

    pub fn with_query(query: &str) -> Self {
        Self {
            input: Input::with_value(query.to_string()),
            history: Vec::new(),
            history_selected: 0,
            history_state: ListState::default(),
            watchlist: Vec::new(),
            watchlist_selected: 0,
            watchlist_state: ListState::default(),
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
            watchlist: Vec::new(),
            watchlist_selected: 0,
            watchlist_state: ListState::default(),
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
            watchlist: Vec::new(),
            watchlist_selected: 0,
            watchlist_state: ListState::default(),
            focus: Focus::Search,
        }
    }

    /// Set the watchlist items
    pub fn set_watchlist(&mut self, watchlist: Vec<WatchlistItem>) {
        self.watchlist = watchlist;
        self.watchlist_selected = 0;
        if !self.watchlist.is_empty() {
            self.watchlist_state.select(Some(0));
        } else {
            self.watchlist_state.select(None);
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

    /// Get the next available focus target when pressing Tab/Down from search
    fn next_focus_from_search(&self) -> Option<Focus> {
        if !self.history.is_empty() {
            Some(Focus::History)
        } else if !self.watchlist.is_empty() {
            Some(Focus::Watchlist)
        } else {
            None
        }
    }

    /// Handle key input, returns Some(action) if an action should be performed
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<SearchAction> {
        match self.focus {
            Focus::Search => self.handle_search_key(key),
            Focus::History => self.handle_history_key(key),
            Focus::Watchlist => self.handle_watchlist_key(key),
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
                // Move focus to the first available list
                if let Some(next) = self.next_focus_from_search() {
                    self.focus = next;
                    match next {
                        Focus::History => {
                            self.history_state.select(Some(self.history_selected));
                        }
                        Focus::Watchlist => {
                            self.watchlist_state.select(Some(self.watchlist_selected));
                        }
                        Focus::Search => {}
                    }
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
            KeyCode::Right | KeyCode::Tab => {
                // Move to watchlist if available
                if !self.watchlist.is_empty() {
                    self.focus = Focus::Watchlist;
                    self.history_state.select(None);
                    self.watchlist_state.select(Some(self.watchlist_selected));
                } else {
                    // Tab back to search
                    self.focus = Focus::Search;
                    self.history_state.select(None);
                }
            }
            KeyCode::Esc => {
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

    fn handle_watchlist_key(&mut self, key: KeyEvent) -> Option<SearchAction> {
        match key.code {
            KeyCode::Enter => {
                if let Some(item) = self.watchlist.get(self.watchlist_selected) {
                    return Some(SearchAction::SelectWatchlist(item.clone()));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.watchlist_selected == 0 {
                    // Move back to search
                    self.focus = Focus::Search;
                    self.watchlist_state.select(None);
                } else {
                    self.watchlist_selected = self.watchlist_selected.saturating_sub(1);
                    self.watchlist_state.select(Some(self.watchlist_selected));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.watchlist_selected < self.watchlist.len().saturating_sub(1) {
                    self.watchlist_selected += 1;
                    self.watchlist_state.select(Some(self.watchlist_selected));
                }
            }
            KeyCode::Left | KeyCode::Tab => {
                // Move to history if available, otherwise search
                if !self.history.is_empty() {
                    self.focus = Focus::History;
                    self.watchlist_state.select(None);
                    self.history_state.select(Some(self.history_selected));
                } else {
                    self.focus = Focus::Search;
                    self.watchlist_state.select(None);
                }
            }
            KeyCode::Char('d') | KeyCode::Char('x') => {
                // Remove from watchlist
                if let Some(item) = self.watchlist.get(self.watchlist_selected) {
                    let item = item.clone();
                    // Remove from local list
                    self.watchlist.remove(self.watchlist_selected);
                    if self.watchlist.is_empty() {
                        self.watchlist_selected = 0;
                        self.watchlist_state.select(None);
                        // Move focus to history or search
                        if !self.history.is_empty() {
                            self.focus = Focus::History;
                            self.history_state.select(Some(self.history_selected));
                        } else {
                            self.focus = Focus::Search;
                        }
                    } else if self.watchlist_selected >= self.watchlist.len() {
                        self.watchlist_selected = self.watchlist.len() - 1;
                        self.watchlist_state.select(Some(self.watchlist_selected));
                    } else {
                        self.watchlist_state.select(Some(self.watchlist_selected));
                    }
                    return Some(SearchAction::RemoveFromWatchlist(item));
                }
            }
            KeyCode::Esc => {
                self.focus = Focus::Search;
                self.watchlist_state.select(None);
            }
            KeyCode::Backspace => {
                self.focus = Focus::Search;
                self.watchlist_state.select(None);
                self.input.backspace();
            }
            _ => {}
        }
        None
    }

    /// Render the search screen
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let has_history = !self.history.is_empty();
        let has_watchlist = !self.watchlist.is_empty();
        let has_lists = has_history || has_watchlist;

        let constraints = if has_lists {
            vec![
                Constraint::Length(3), // Title
                Constraint::Length(3), // Input
                Constraint::Length(2), // Help text
                Constraint::Length(1), // Spacer
                Constraint::Min(5),    // Lists
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
        let help = if has_lists {
            let mut spans = vec![
                Span::styled("Enter", theme.highlight()),
                Span::styled(" search ", theme.muted()),
                Span::styled("Tab", theme.highlight()),
            ];
            if has_history && has_watchlist {
                spans.push(Span::styled(" navigate lists ", theme.muted()));
            } else if has_history {
                spans.push(Span::styled(" history ", theme.muted()));
            } else {
                spans.push(Span::styled(" watchlist ", theme.muted()));
            }
            if has_watchlist {
                spans.push(Span::styled("d", theme.highlight()));
                spans.push(Span::styled(" remove ", theme.muted()));
            }
            spans.push(Span::styled("^T", theme.highlight()));
            spans.push(Span::styled(" theme ", theme.muted()));
            spans.push(Span::styled("Esc", theme.highlight()));
            spans.push(Span::styled(" quit", theme.muted()));
            Line::from(spans)
        } else {
            Line::from(vec![
                Span::styled("Enter", theme.highlight()),
                Span::styled(" search ", theme.muted()),
                Span::styled("^T", theme.highlight()),
                Span::styled(" theme ", theme.muted()),
                Span::styled("Esc", theme.highlight()),
                Span::styled(" quit", theme.muted()),
            ])
        };
        let help_widget = Paragraph::new(help);
        frame.render_widget(help_widget, chunks[2]);

        // Lists section
        if has_lists {
            let list_area = chunks[4];

            if has_history && has_watchlist {
                // Side by side
                let columns = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(list_area);

                self.render_history(frame, columns[0], theme);
                self.render_watchlist(frame, columns[1], theme);
            } else if has_history {
                self.render_history(frame, list_area, theme);
            } else {
                self.render_watchlist(frame, list_area, theme);
            }
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

    fn render_watchlist(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self
            .watchlist
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let is_selected = self.focus == Focus::Watchlist && i == self.watchlist_selected;

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

                // Media type badge
                let type_badge = match item.media_type {
                    crate::api::MediaType::Movie => " [Movie]",
                    crate::api::MediaType::TvShow => " [TV]",
                };
                spans.push(Span::styled(type_badge, theme.muted()));

                // Time added
                spans.push(Span::styled(
                    format!(" - {}", item.added_at_display()),
                    theme.muted(),
                ));

                ListItem::new(Line::from(spans))
            })
            .collect();

        let border_style = if self.focus == Focus::Watchlist {
            theme.highlight()
        } else {
            theme.border()
        };

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(" Watchlist "),
        );

        frame.render_stateful_widget(list, area, &mut self.watchlist_state);
    }
}

impl Default for SearchScreen {
    fn default() -> Self {
        Self::new()
    }
}
