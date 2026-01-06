use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::api::{Episode, Media, Season};
use crate::ui::components::SelectableList;
use crate::ui::theme::Theme;

/// Action from episodes screen
pub enum EpisodesAction {
    Select(Episode),
    Back,
    /// Toggle watched status for an episode
    ToggleWatched(Episode),
}

/// Episode selection screen
pub struct EpisodesScreen {
    pub media: Media,
    pub season: Option<Season>,
    pub list: SelectableList<Episode>,
    /// Set of watched episode numbers
    watched_episodes: HashSet<u32>,
}

impl EpisodesScreen {
    /// Create episode screen for anime (no season needed)
    pub fn new(media: Media) -> Self {
        let episodes = media.get_episodes();
        Self {
            list: SelectableList::new(episodes),
            media,
            season: None,
            watched_episodes: HashSet::new(),
        }
    }

    /// Create episode screen for a specific season (TV shows)
    pub fn with_season(media: Media, season: Season) -> Self {
        let episodes = season.get_episodes();
        Self {
            list: SelectableList::new(episodes),
            media,
            season: Some(season),
            watched_episodes: HashSet::new(),
        }
    }

    /// Set watched episodes
    pub fn set_watched_episodes(&mut self, watched: HashSet<u32>) {
        self.watched_episodes = watched;
    }

    /// Check if an episode is watched
    pub fn is_watched(&self, episode_number: u32) -> bool {
        self.watched_episodes.contains(&episode_number)
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
            KeyCode::Char('w') => {
                // Toggle watched status
                if let Some(episode) = self.list.get_selected() {
                    return Some(EpisodesAction::ToggleWatched(episode.clone()));
                }
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                return Some(EpisodesAction::Back);
            }
            _ => {}
        }
        None
    }

    /// Toggle the watched status of an episode locally
    pub fn toggle_watched(&mut self, episode_number: u32) {
        if self.watched_episodes.contains(&episode_number) {
            self.watched_episodes.remove(&episode_number);
        } else {
            self.watched_episodes.insert(episode_number);
        }
    }

    /// Get the season number (defaults to 1 for anime)
    pub fn season_number(&self) -> u32 {
        self.season.as_ref().map(|s| s.number).unwrap_or(1)
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

        // Title - show season info if available
        let watched_count = self.watched_episodes.len();
        let total_count = self.list.len();

        let title = if let Some(season) = &self.season {
            let mut spans = vec![
                Span::styled(self.media.display_title(), theme.title()),
                Span::styled(format!(" - Season {}", season.number), theme.highlight()),
            ];
            if watched_count > 0 {
                spans.push(Span::styled(
                    format!(" ({}/{} watched)", watched_count, total_count),
                    theme.muted(),
                ));
            } else {
                spans.push(Span::styled(
                    format!(" ({} episodes)", total_count),
                    theme.muted(),
                ));
            }
            Line::from(spans)
        } else {
            let mut spans = vec![Span::styled(self.media.display_title(), theme.title())];
            if watched_count > 0 {
                spans.push(Span::styled(
                    format!(" ({}/{} watched)", watched_count, total_count),
                    theme.muted(),
                ));
            } else {
                spans.push(Span::styled(
                    format!(" ({} episodes)", total_count),
                    theme.muted(),
                ));
            }
            Line::from(spans)
        };
        let title_widget = Paragraph::new(title);
        frame.render_widget(title_widget, chunks[0]);

        // Episodes list
        if self.list.is_empty() {
            let no_episodes = Paragraph::new(Line::from(vec![Span::styled(
                "No episodes found.",
                theme.warning(),
            )]));
            frame.render_widget(no_episodes, chunks[1]);
        } else {
            // Clone watched_episodes for the closure
            let watched = self.watched_episodes.clone();

            self.list.render(
                frame,
                chunks[1],
                " Episodes ",
                theme,
                |episode, is_selected| {
                    let is_watched = watched.contains(&episode.number);

                    let style = if is_selected {
                        theme.selected()
                    } else if is_watched {
                        theme.muted()
                    } else {
                        theme.normal()
                    };
                    let muted = theme.muted();

                    let mut spans = vec![];

                    // Watched indicator
                    if is_watched {
                        spans.push(Span::styled("[x] ", theme.success()));
                    } else {
                        spans.push(Span::styled("[ ] ", muted));
                    }

                    spans.push(Span::styled(format!("{}. ", episode.number), muted));
                    spans.push(Span::styled(episode.title.clone(), style));

                    spans
                },
            );
        }

        // Help text
        let help = Line::from(vec![
            Span::styled("^/v", theme.highlight()),
            Span::styled(" navigate ", theme.muted()),
            Span::styled("Enter", theme.highlight()),
            Span::styled(" play ", theme.muted()),
            Span::styled("w", theme.highlight()),
            Span::styled(" toggle watched ", theme.muted()),
            Span::styled("Esc", theme.highlight()),
            Span::styled(" back", theme.muted()),
        ]);
        let help_widget = Paragraph::new(help);
        frame.render_widget(help_widget, chunks[2]);
    }
}
