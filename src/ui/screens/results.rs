use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::api::{Media, MediaType};
use crate::ui::components::SelectableList;
use crate::ui::theme::{Theme, STAR};

/// Action from results screen
pub enum ResultsAction {
    Select(Media),
    Back,
    Search,
}

/// Search results screen for all media types
pub struct ResultsScreen {
    pub query: String,
    pub list: SelectableList<Media>,
}

impl ResultsScreen {
    pub fn new(query: String, results: Vec<Media>) -> Self {
        Self {
            query,
            list: SelectableList::new(results),
        }
    }

    /// Handle key input
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<ResultsAction> {
        match key.code {
            KeyCode::Enter => {
                if let Some(media) = self.list.get_selected() {
                    return Some(ResultsAction::Select(media.clone()));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.list.previous();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.list.next();
            }
            KeyCode::Esc => {
                return Some(ResultsAction::Back);
            }
            KeyCode::Char('/') => {
                return Some(ResultsAction::Search);
            }
            _ => {}
        }
        None
    }

    /// Render the results screen
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Min(5),    // Results list
                Constraint::Length(2), // Help text
            ])
            .margin(1)
            .split(area);

        // Title
        let title = Line::from(vec![
            Span::styled("Results for ", theme.muted()),
            Span::styled(format!("\"{}\"", self.query), theme.highlight()),
            Span::styled(format!(" ({} found)", self.list.len()), theme.muted()),
        ]);
        let title_widget = Paragraph::new(title);
        frame.render_widget(title_widget, chunks[0]);

        // Results list
        if self.list.is_empty() {
            let no_results = Paragraph::new(Line::from(vec![
                Span::styled("No results found. ", theme.warning()),
                Span::styled("Try a different search term.", theme.muted()),
            ]));
            frame.render_widget(no_results, chunks[1]);
        } else {
            self.list.render(frame, chunks[1], " Results ", theme, |media, is_selected| {
                let style = if is_selected { theme.selected() } else { theme.normal() };
                let muted = theme.muted();

                // Media type indicator
                let type_style = match media.media_type {
                    MediaType::Anime => theme.highlight(),
                    MediaType::Movie => theme.accent(),
                    MediaType::TvShow => theme.info(),
                };

                let mut spans = vec![
                    Span::styled(format!("[{}] ", media.media_type.label()), type_style),
                    Span::styled(media.display_title().to_string(), style),
                ];

                if let Some(score) = media.score {
                    if score > 0.0 {
                        spans.push(Span::styled(format!("  {} {:.1}", STAR, score), muted));
                    }
                }

                if let Some(year) = media.year {
                    spans.push(Span::styled(format!("  {}", year), muted));
                }

                // Show episode/season count based on media type
                match media.media_type {
                    MediaType::Anime => {
                        if let Some(eps) = media.episodes {
                            spans.push(Span::styled(format!("  ({} eps)", eps), muted));
                        }
                    }
                    MediaType::TvShow => {
                        if let Some(seasons) = media.seasons {
                            spans.push(Span::styled(format!("  ({} seasons)", seasons), muted));
                        }
                    }
                    MediaType::Movie => {}
                }

                spans
            });
        }

        // Help text
        let help = Line::from(vec![
            Span::styled("↑/↓", theme.highlight()),
            Span::styled(" navigate • ", theme.muted()),
            Span::styled("Enter", theme.highlight()),
            Span::styled(" select • ", theme.muted()),
            Span::styled("/", theme.highlight()),
            Span::styled(" search • ", theme.muted()),
            Span::styled("Esc", theme.highlight()),
            Span::styled(" back", theme.muted()),
        ]);
        let help_widget = Paragraph::new(help);
        frame.render_widget(help_widget, chunks[2]);
    }
}
