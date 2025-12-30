use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::api::Stream;
use crate::ui::components::{SelectableList, StreamDetailCard};
use crate::ui::theme::Theme;

/// Minimum terminal width to show the detail card
const MIN_WIDTH_FOR_DETAIL_CARD: u16 = 100;

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
        let show_detail_card = area.width >= MIN_WIDTH_FOR_DETAIL_CARD && !self.list.is_empty();

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Min(5),    // Sources list (and detail card)
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

        // Main content area - split horizontally if wide enough
        if self.list.is_empty() {
            let no_sources = Paragraph::new(Line::from(vec![
                Span::styled("No sources found. ", theme.warning()),
                Span::styled("Try a different title.", theme.muted()),
            ]));
            frame.render_widget(no_sources, chunks[1]);
        } else if show_detail_card {
            // Two-column layout: list on left, detail card on right
            let content_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(55), // Sources list
                    Constraint::Percentage(45), // Detail card
                ])
                .split(chunks[1]);

            // Render the list
            self.render_list(frame, content_chunks[0], theme);

            // Render the detail card for the selected item
            if let Some(stream) = self.list.get_selected() {
                StreamDetailCard::render(frame, content_chunks[1], stream, theme);
            }
        } else {
            // Single column layout - just the list
            self.render_list(frame, chunks[1], theme);
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

    /// Render the sources list
    fn render_list(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.list.render(frame, area, " Select Source ", theme, |source, is_selected| {
            let style = if is_selected { theme.selected() } else { theme.normal() };
            let muted = theme.muted();

            let mut spans = vec![
                Span::styled(source.provider.clone(), style),
            ];

            if let Some(quality) = &source.quality {
                spans.push(Span::styled(format!(" {}", quality), style));
            }

            // Show HDR info in list if available
            if let Some(hdr) = &source.hdr {
                spans.push(Span::styled(format!(" {}", hdr), theme.warning()));
            }

            if let Some(size) = &source.size {
                spans.push(Span::styled(format!(" {}", size), muted));
            }

            if let Some(seeders) = source.seeders {
                spans.push(Span::styled(format!(" 👤{}", seeders), muted));
            }

            // Show languages if available
            if !source.languages.is_empty() {
                let lang_display = if source.languages.len() <= 2 {
                    source.languages.join(", ")
                } else {
                    format!("{} langs", source.languages.len())
                };
                spans.push(Span::styled(format!(" ({})", lang_display), muted));
            }

            spans
        });
    }
}
