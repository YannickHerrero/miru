use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::api::{Anime, Episode};
use crate::ui::components::SelectableList;
use crate::ui::theme::Theme;

/// Action from episodes screen
pub enum EpisodesAction {
    Select(Episode),
    Back,
}

/// Episode selection screen
pub struct EpisodesScreen {
    pub anime: Anime,
    pub list: SelectableList<Episode>,
}

impl EpisodesScreen {
    pub fn new(anime: Anime) -> Self {
        let episodes = anime.get_episodes();
        Self {
            list: SelectableList::new(episodes),
            anime,
        }
    }

    /// Handle key input
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<EpisodesAction> {
        match key.code {
            KeyCode::Enter => {
                if let Some(episode) = self.list.get_selected() {
                    return Some(EpisodesAction::Select(episode.clone()));
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.list.previous();
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.list.next();
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                return Some(EpisodesAction::Back);
            }
            _ => {}
        }
        None
    }

    /// Render the episodes screen
    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Title
                Constraint::Min(5),    // Episodes list
                Constraint::Length(2), // Help text
            ])
            .margin(1)
            .split(area);

        // Title
        let title = Line::from(vec![
            Span::styled(self.anime.display_title(), theme.title()),
            Span::styled(
                format!(" ({} episodes)", self.list.len()),
                theme.muted(),
            ),
        ]);
        let title_widget = Paragraph::new(title);
        frame.render_widget(title_widget, chunks[0]);

        // Episodes list
        if self.list.is_empty() {
            let no_episodes = Paragraph::new(Line::from(vec![
                Span::styled("No episodes found.", theme.warning()),
            ]));
            frame.render_widget(no_episodes, chunks[1]);
        } else {
            self.list.render(frame, chunks[1], " Episodes ", theme, |episode, is_selected| {
                let style = if is_selected { theme.selected() } else { theme.normal() };
                let muted = theme.muted();

                vec![
                    Span::styled(format!("{}. ", episode.number), muted),
                    Span::styled(episode.title.clone(), style),
                ]
            });
        }

        // Help text
        let help = Line::from(vec![
            Span::styled("↑/↓", theme.highlight()),
            Span::styled(" navigate • ", theme.muted()),
            Span::styled("Enter", theme.highlight()),
            Span::styled(" select • ", theme.muted()),
            Span::styled("Esc", theme.highlight()),
            Span::styled(" back", theme.muted()),
        ]);
        let help_widget = Paragraph::new(help);
        frame.render_widget(help_widget, chunks[2]);
    }
}
