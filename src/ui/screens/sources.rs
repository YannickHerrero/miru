use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::api::{Media, Stream};
use crate::ui::components::{SelectableList, StreamDetailCard};
use crate::ui::theme::Theme;

/// Minimum terminal width to show the detail card
const MIN_WIDTH_FOR_DETAIL_CARD: u16 = 100;

/// Action from sources screen
pub enum SourcesAction {
    Select(Stream),
    Back,
    ToggleUncached,
}

/// Context needed to re-fetch sources
#[derive(Clone)]
pub struct SourcesContext {
    pub media: Media,
    pub season: u32,
    pub episode: u32,
    pub imdb_id: String,
}

/// Source/torrent selection screen
pub struct SourcesScreen {
    pub title: String,
    pub episode_number: Option<u32>,
    pub list: SelectableList<Stream>,
    /// Whether uncached sources are currently shown
    pub show_uncached: bool,
    /// Context for re-fetching sources when toggling
    pub context: SourcesContext,
}

impl SourcesScreen {
    pub fn new(
        title: String,
        episode_number: u32,
        sources: Vec<Stream>,
        context: SourcesContext,
        show_uncached: bool,
    ) -> Self {
        Self {
            title,
            episode_number: if episode_number > 0 {
                Some(episode_number)
            } else {
                None
            },
            list: SelectableList::new(sources),
            show_uncached,
            context,
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
            KeyCode::Char('u') => {
                return Some(SourcesAction::ToggleUncached);
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

        // Title with uncached indicator
        let mut title_spans = vec![];
        if let Some(ep) = self.episode_number {
            title_spans.push(Span::styled(&self.title, theme.title()));
            title_spans.push(Span::styled(format!(" - Episode {}", ep), theme.muted()));
        } else {
            title_spans.push(Span::styled(&self.title, theme.title()));
        }
        
        // Show uncached indicator in title
        if self.show_uncached {
            title_spans.push(Span::styled(" [showing uncached]", theme.warning()));
        }
        
        let title = Line::from(title_spans);
        let title_widget = Paragraph::new(title);
        frame.render_widget(title_widget, chunks[0]);

        // Main content area - split horizontally if wide enough
        if self.list.is_empty() {
            self.render_empty_state(frame, chunks[1], theme);
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
        self.render_help(frame, chunks[2], theme);
    }

    /// Render the empty state message
    fn render_empty_state(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let mut lines = vec![];
        
        lines.push(Line::from(vec![
            Span::styled("No sources found", theme.warning()),
        ]));
        
        lines.push(Line::from(""));
        
        if !self.show_uncached {
            lines.push(Line::from(vec![
                Span::styled("Press ", theme.muted()),
                Span::styled("u", theme.highlight()),
                Span::styled(" to show uncached sources", theme.muted()),
            ]));
        } else {
            lines.push(Line::from(vec![
                Span::styled("No torrents available for this title.", theme.muted()),
            ]));
        }
        
        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, area);
    }

    /// Render the help text
    fn render_help(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let uncached_text = if self.show_uncached {
            "hide uncached"
        } else {
            "show uncached"
        };
        
        let help = Line::from(vec![
            Span::styled("↑/↓", theme.highlight()),
            Span::styled(" navigate • ", theme.muted()),
            Span::styled("Enter", theme.highlight()),
            Span::styled(" play • ", theme.muted()),
            Span::styled("u", theme.highlight()),
            Span::styled(format!(" {} • ", uncached_text), theme.muted()),
            Span::styled("Esc", theme.highlight()),
            Span::styled(" back", theme.muted()),
        ]);
        let help_widget = Paragraph::new(help);
        frame.render_widget(help_widget, area);
    }

    /// Render the sources list
    fn render_list(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.list.render(frame, area, " Select Source ", theme, |source, is_selected| {
            let style = if is_selected { theme.selected() } else { theme.normal() };
            let muted = theme.muted();

            let mut spans = vec![];
            
            // Show uncached indicator
            if !source.is_cached {
                spans.push(Span::styled("[uncached] ", theme.error()));
            }

            if let Some(quality) = &source.quality {
                spans.push(Span::styled(format!("[{}]", quality), style));
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
