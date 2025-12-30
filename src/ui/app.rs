use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::Context;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Frame, Terminal,
};

use crate::api::{Anime, AnilistClient, Episode, MappingClient, RealDebridClient, Stream, TorrentioClient};
use crate::config::Config;
use crate::error::Result;
use crate::player::Player;
use crate::ui::components::Spinner;
use crate::ui::screens::{
    EpisodesAction, EpisodesScreen, ErrorAction, ErrorScreen, ResultsAction, ResultsScreen,
    SearchScreen, SourcesAction, SourcesScreen,
};
use crate::ui::theme::Theme;

/// Application state
enum Screen {
    Search(SearchScreen),
    Results(ResultsScreen),
    Episodes(EpisodesScreen),
    Sources(SourcesScreen),
    Loading(Spinner),
    Error(ErrorScreen),
}

/// Pending async operation
enum PendingOperation {
    None,
    Search(String),
    FetchEpisodes(Anime),
    FetchSources(Anime, Episode),
    ResolveStream(Anime, Episode, Stream),
}

/// Main TUI application
pub struct App {
    #[allow(dead_code)]
    config: Config,
    screen: Screen,
    pending: PendingOperation,
    should_quit: bool,
    // API clients
    anilist: AnilistClient,
    mapping: MappingClient,
    torrentio: TorrentioClient,
    #[allow(dead_code)]
    realdebrid: RealDebridClient,
    player: Player,
    // Theme
    theme: Theme,
}

impl App {
    pub fn new(config: Config) -> Self {
        let torrentio = TorrentioClient::new(
            config.torrentio.clone(),
            config.real_debrid.api_key.clone(),
        );
        let realdebrid = RealDebridClient::new(config.real_debrid.api_key.clone());
        let player = Player::new(config.player.clone());

        Self {
            config,
            screen: Screen::Search(SearchScreen::new()),
            pending: PendingOperation::None,
            should_quit: false,
            anilist: AnilistClient::new(),
            mapping: MappingClient::new(),
            torrentio,
            realdebrid,
            player,
            theme: Theme::default(),
        }
    }

    /// Set an initial search query
    pub fn set_initial_query(&mut self, query: &str) {
        self.screen = Screen::Search(SearchScreen::with_query(query));
    }

    /// Run the TUI application
    pub async fn run(&mut self) -> Result<()> {
        let mut terminal = self.setup_terminal()?;

        while !self.should_quit {
            // Render current screen
            terminal.draw(|f| self.render(f))?;

            // Handle pending operations
            if !matches!(self.pending, PendingOperation::None) {
                self.handle_pending_operation().await;
                continue;
            }

            // Poll for events with a timeout (for spinner animation)
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    // Only handle key press events, not release
                    if key.kind == KeyEventKind::Press {
                        self.handle_key_event(key, &mut terminal)?;
                    }
                }
            }
        }

        self.restore_terminal(&mut terminal)?;
        Ok(())
    }

    fn setup_terminal(&self) -> Result<Terminal<CrosstermBackend<Stdout>>> {
        enable_raw_mode().context("Failed to enable raw mode")?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).context("Failed to create terminal")?;
        Ok(terminal)
    }

    fn restore_terminal(&self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        disable_raw_mode().context("Failed to disable raw mode")?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)
            .context("Failed to leave alternate screen")?;
        terminal.show_cursor().context("Failed to show cursor")?;
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        match &mut self.screen {
            Screen::Search(screen) => screen.render(frame, area, &self.theme),
            Screen::Results(screen) => screen.render(frame, area, &self.theme),
            Screen::Episodes(screen) => screen.render(frame, area, &self.theme),
            Screen::Sources(screen) => screen.render(frame, area, &self.theme),
            Screen::Loading(spinner) => {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Percentage(45),
                        Constraint::Length(3),
                        Constraint::Min(0),
                    ])
                    .split(area);
                spinner.render(frame, chunks[1], &self.theme);
            }
            Screen::Error(screen) => screen.render(frame, area, &self.theme),
        }
    }

    fn handle_key_event(
        &mut self,
        key: crossterm::event::KeyEvent,
        _terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<()> {
        // Global quit handler
        if key.code == KeyCode::Char('q') && matches!(self.screen, Screen::Search(_)) {
            self.should_quit = true;
            return Ok(());
        }
        if key.code == KeyCode::Esc && matches!(self.screen, Screen::Search(_)) {
            self.should_quit = true;
            return Ok(());
        }

        match &mut self.screen {
            Screen::Search(screen) => {
                if let Some(query) = screen.handle_key(key) {
                    self.pending = PendingOperation::Search(query.clone());
                    self.screen = Screen::Loading(Spinner::new("Searching Anilist..."));
                }
            }
            Screen::Results(screen) => {
                if let Some(action) = screen.handle_key(key) {
                    match action {
                        ResultsAction::Select(anime) => {
                            self.pending = PendingOperation::FetchEpisodes(anime.clone());
                            self.screen = Screen::Loading(Spinner::new("Loading episodes..."));
                        }
                        ResultsAction::Back => {
                            self.screen = Screen::Search(SearchScreen::with_query(&screen.query));
                        }
                        ResultsAction::Search => {
                            self.screen = Screen::Search(SearchScreen::new());
                        }
                    }
                }
            }
            Screen::Episodes(screen) => {
                if let Some(action) = screen.handle_key(key) {
                    match action {
                        EpisodesAction::Select(episode) => {
                            self.pending =
                                PendingOperation::FetchSources(screen.anime.clone(), episode);
                            self.screen = Screen::Loading(Spinner::new("Fetching sources..."));
                        }
                        EpisodesAction::Back => {
                            // Go back to results would require re-searching, so we just go to search
                            self.screen = Screen::Search(SearchScreen::new());
                        }
                    }
                }
            }
            Screen::Sources(screen) => {
                if let Some(action) = screen.handle_key(key) {
                    match action {
                        SourcesAction::Select(stream) => {
                            // We need to reconstruct the anime and episode - store them in Sources screen
                            // For now, we'll use placeholders
                            self.pending = PendingOperation::ResolveStream(
                                Anime {
                                    id: 0,
                                    id_mal: None,
                                    title: screen.anime_title.clone(),
                                    title_english: None,
                                    title_native: None,
                                    episodes: None,
                                    score: None,
                                    year: None,
                                    status: None,
                                    format: None,
                                    cover_image: None,
                                    episode_titles: vec![],
                                },
                                Episode {
                                    number: screen.episode_number,
                                    title: String::new(),
                                },
                                stream,
                            );
                            self.screen = Screen::Loading(Spinner::new("Resolving stream..."));
                        }
                        SourcesAction::Back => {
                            self.screen = Screen::Search(SearchScreen::new());
                        }
                    }
                }
            }
            Screen::Loading(_) => {
                // Allow cancelling with Esc
                if key.code == KeyCode::Esc {
                    self.pending = PendingOperation::None;
                    self.screen = Screen::Search(SearchScreen::new());
                }
            }
            Screen::Error(screen) => {
                if let Some(action) = screen.handle_key(key) {
                    match action {
                        ErrorAction::Retry => {
                            // Could implement retry logic here
                            self.screen = Screen::Search(SearchScreen::new());
                        }
                        ErrorAction::Back => {
                            self.screen = Screen::Search(SearchScreen::new());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_pending_operation(&mut self) {
        let operation = std::mem::replace(&mut self.pending, PendingOperation::None);

        match operation {
            PendingOperation::None => {}
            PendingOperation::Search(query) => {
                match self.anilist.search_anime(&query).await {
                    Ok(results) => {
                        self.screen = Screen::Results(ResultsScreen::new(query, results));
                    }
                    Err(e) => {
                        self.screen = Screen::Error(ErrorScreen::new(e.to_string(), true));
                    }
                }
            }
            PendingOperation::FetchEpisodes(anime) => {
                // We already have episode count from search, so just show episodes
                self.screen = Screen::Episodes(EpisodesScreen::new(anime));
            }
            PendingOperation::FetchSources(anime, episode) => {
                // Get IMDB ID
                let imdb_result = self
                    .mapping
                    .anilist_to_imdb(anime.id, anime.id_mal)
                    .await;

                match imdb_result {
                    Ok(imdb_id) => {
                        // Fetch streams from Torrentio
                        // Assume season 1 for now (anime typically uses season 1)
                        match self
                            .torrentio
                            .get_streams(&imdb_id, 1, episode.number)
                            .await
                        {
                            Ok(streams) => {
                                self.screen = Screen::Sources(SourcesScreen::new(
                                    anime.display_title().to_string(),
                                    episode.number,
                                    streams,
                                ));
                            }
                            Err(e) => {
                                self.screen =
                                    Screen::Error(ErrorScreen::new(e.to_string(), true));
                            }
                        }
                    }
                    Err(e) => {
                        self.screen = Screen::Error(ErrorScreen::new(e.to_string(), false));
                    }
                }
            }
            PendingOperation::ResolveStream(_anime, _episode, stream) => {
                // With debridoptions=nodownloadlinks, Torrentio only returns cached streams
                // with direct Real-Debrid URLs that can be played immediately
                let url = if let Some(url) = &stream.url {
                    // Direct RD URL - use as-is, no need to unrestrict
                    Some(url.clone())
                } else {
                    self.screen = Screen::Error(ErrorScreen::new(
                        "No URL available for this source",
                        false,
                    ));
                    return;
                };

                if let Some(url) = url {
                    // Launch player
                    // First restore terminal
                    disable_raw_mode().ok();
                    execute!(io::stdout(), LeaveAlternateScreen).ok();

                    match self.player.play(&url) {
                        Ok(()) => {
                            // Player finished, restore TUI
                            enable_raw_mode().ok();
                            execute!(io::stdout(), EnterAlternateScreen).ok();
                            // Go back to sources screen
                            self.screen = Screen::Search(SearchScreen::new());
                        }
                        Err(e) => {
                            // Restore TUI even on error
                            enable_raw_mode().ok();
                            execute!(io::stdout(), EnterAlternateScreen).ok();
                            self.screen = Screen::Error(ErrorScreen::new(e.to_string(), false));
                        }
                    }
                }
            }
        }
    }
}
