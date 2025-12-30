use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::api::Stream;
use crate::ui::components::SelectableList;
use crate::ui::theme::Theme;

/// Action from sources screen
pub enum SourcesAction {
    Select(Stream),
    Back,
}

/// Source/torrent selection screen
pub struct SourcesScreen {
    pub title: String,
    pub episode_number: Option<u32>,
    pub list: SelectableList<Stream>,
}

impl SourcesScreen {
    pub fn new(title: String, episode_number: u32, sources: Vec<Stream>) -> Self {
        Self {
            title,
            episode_number: if episode_number > 0 {
                Some(episode_number)
            } else {
                None
            },
            list: SelectableList::new(sources),
        }
    }

    /// Handle key input
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<SourcesAction> {
        match key.code {
            KeyCode::Enter => {
                if let Some(source) = self.list.get_selected() {
                    return Some(SourcesAction::Select(source.clone()));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.list.previous();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.list.next();
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                return Some(SourcesAction::Back);
            }
            _ => {}
        }
        None
    }

    /// Render the sources screen
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Min(5),    // Sources list
                Constraint::Length(2), // Help text
            ])
            .margin(1)
            .split(area);

        // Title
        let title = if let Some(ep) = self.episode_number {
            Line::from(vec![
                Span::styled(&self.title, theme.title()),
                Span::styled(format!(" - Episode {}", ep), theme.muted()),
            ])
        } else {
            Line::from(vec![Span::styled(&self.title, theme.title())])
        };
        let title_widget = Paragraph::new(title);
        frame.render_widget(title_widget, chunks[0]);

        // Sources list
        if self.list.is_empty() {
            let no_sources = Paragraph::new(Line::from(vec![
                Span::styled("No sources found. ", theme.warning()),
                Span::styled("Try a different title.", theme.muted()),
            ]));
            frame.render_widget(no_sources, chunks[1]);
        } else {
            self.list.render(frame, chunks[1], " Select Source ", theme, |source, is_selected| {
                let style = if is_selected { theme.selected() } else { theme.normal() };
                let muted = theme.muted();

                let mut spans = vec![
                    Span::styled(format!("[{}]", source.provider), style),
                ];

                if let Some(quality) = &source.quality {
                    spans.push(Span::styled(format!(" {}", quality), style));
                }

                if let Some(size) = &source.size {
                    spans.push(Span::styled(format!(" {}", size), muted));
                }

                if let Some(seeders) = source.seeders {
                    spans.push(Span::styled(format!(" 👤{}", seeders), muted));
                }

                spans
            });
        }

        // Help text
        let help = Line::from(vec![
            Span::styled("↑/↓", theme.highlight()),
            Span::styled(" navigate • ", theme.muted()),
            Span::styled("Enter", theme.highlight()),
            Span::styled(" play • ", theme.muted()),
            Span::styled("Esc", theme.highlight()),
            Span::styled(" back", theme.muted()),
        ]);
        let help_widget = Paragraph::new(help);
        frame.render_widget(help_widget, chunks[2]);
    }
}
