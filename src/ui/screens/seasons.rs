use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::api::{Media, Season};
use crate::ui::components::SelectableList;
use crate::ui::theme::Theme;

/// Action from seasons screen
pub enum SeasonsAction {
    Select(Season),
    Back,
}

/// Season selection screen for TV shows
pub struct SeasonsScreen {
    pub media: Media,
    pub list: SelectableList<Season>,
}

impl SeasonsScreen {
    pub fn new(media: Media, seasons: Vec<Season>) -> Self {
        Self {
            list: SelectableList::new(seasons),
            media,
        }
    }

    /// Handle key input
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<SeasonsAction> {
        match key.code {
            KeyCode::Enter => {
                if let Some(season) = self.list.get_selected() {
                    return Some(SeasonsAction::Select(season.clone()));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.list.previous();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.list.next();
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                return Some(SeasonsAction::Back);
            }
            _ => {}
        }
        None
    }

    /// Render the seasons screen
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Min(5),    // Seasons list
                Constraint::Length(2), // Help text
            ])
            .margin(1)
            .split(area);

        // Title
        let title = Line::from(vec![
            Span::styled(self.media.display_title(), theme.title()),
            Span::styled(
                format!(" ({} seasons)", self.list.len()),
                theme.muted(),
            ),
        ]);
        let title_widget = Paragraph::new(title);
        frame.render_widget(title_widget, chunks[0]);

        // Seasons list
        if self.list.is_empty() {
            let no_seasons = Paragraph::new(Line::from(vec![
                Span::styled("No seasons found.", theme.warning()),
            ]));
            frame.render_widget(no_seasons, chunks[1]);
        } else {
            self.list.render(frame, chunks[1], " Seasons ", theme, |season, is_selected| {
                let style = if is_selected { theme.selected() } else { theme.normal() };
                let muted = theme.muted();

                vec![
                    Span::styled(format!("Season {} ", season.number), style),
                    Span::styled(format!("({} episodes)", season.episode_count), muted),
                ]
            });
        }

        // Help text
        let help = Line::from(vec![
            Span::styled("^/v", theme.highlight()),
            Span::styled(" navigate ", theme.muted()),
            Span::styled("Enter", theme.highlight()),
            Span::styled(" select ", theme.muted()),
            Span::styled("Esc", theme.highlight()),
            Span::styled(" back", theme.muted()),
        ]);
        let help_widget = Paragraph::new(help);
        frame.render_widget(help_widget, chunks[2]);
    }
}
