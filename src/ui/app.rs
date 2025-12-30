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

use crate::api::{
    AnilistClient, MappingClient, Media, MediaSource, MediaType, RealDebridClient, Season, Stream,
    TmdbClient, TorrentioClient,
};
use crate::config::Config;
use crate::error::Result;
use crate::player::Player;
use crate::ui::components::Spinner;
use crate::ui::screens::{
    EpisodesAction, EpisodesScreen, ErrorAction, ErrorScreen, ResultsAction, ResultsScreen,
    SearchScreen, SeasonsAction, SeasonsScreen, SourcesAction, SourcesScreen,
};
use crate::ui::theme::Theme;

/// Application state
enum Screen {
    Search(SearchScreen),
    Results(ResultsScreen),
    Seasons(SeasonsScreen),
    Episodes(EpisodesScreen),
    Sources(SourcesScreen),
    Loading(Spinner),
    Error(ErrorScreen),
}

/// Pending async operation
enum PendingOperation {
    None,
    Search(String),
    SelectMedia(Media),
    FetchSeasons(Media),
    FetchEpisodes(Media, Option<Season>),
    FetchSources {
        media: Media,
        season: u32,
        episode: u32,
    },
    ResolveStream(Stream),
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
    tmdb: TmdbClient,
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
        let tmdb = TmdbClient::new(config.tmdb.api_key.clone());
        let player = Player::new(config.player.clone());

        Self {
            config,
            screen: Screen::Search(SearchScreen::new()),
            pending: PendingOperation::None,
            should_quit: false,
            anilist: AnilistClient::new(),
            tmdb,
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
            Screen::Seasons(screen) => screen.render(frame, area, &self.theme),
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
                    self.screen = Screen::Loading(Spinner::new("Searching..."));
                }
            }
            Screen::Results(screen) => {
                if let Some(action) = screen.handle_key(key) {
                    match action {
                        ResultsAction::Select(media) => {
                            self.pending = PendingOperation::SelectMedia(media);
                            self.screen = Screen::Loading(Spinner::new("Loading..."));
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
            Screen::Seasons(screen) => {
                if let Some(action) = screen.handle_key(key) {
                    match action {
                        SeasonsAction::Select(season) => {
                            self.pending =
                                PendingOperation::FetchEpisodes(screen.media.clone(), Some(season));
                            self.screen = Screen::Loading(Spinner::new("Loading episodes..."));
                        }
                        SeasonsAction::Back => {
                            self.screen = Screen::Search(SearchScreen::new());
                        }
                    }
                }
            }
            Screen::Episodes(screen) => {
                if let Some(action) = screen.handle_key(key) {
                    match action {
                        EpisodesAction::Select(episode) => {
                            let season_num = screen.season_number();
                            self.pending = PendingOperation::FetchSources {
                                media: screen.media.clone(),
                                season: season_num,
                                episode: episode.number,
                            };
                            self.screen = Screen::Loading(Spinner::new("Fetching sources..."));
                        }
                        EpisodesAction::Back => {
                            self.screen = Screen::Search(SearchScreen::new());
                        }
                    }
                }
            }
            Screen::Sources(screen) => {
                if let Some(action) = screen.handle_key(key) {
                    match action {
                        SourcesAction::Select(stream) => {
                            self.pending = PendingOperation::ResolveStream(stream);
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
                self.handle_search(&query).await;
            }

            PendingOperation::SelectMedia(media) => {
                self.handle_select_media(media).await;
            }

            PendingOperation::FetchSeasons(media) => {
                self.handle_fetch_seasons(media).await;
            }

            PendingOperation::FetchEpisodes(media, season) => {
                self.handle_fetch_episodes(media, season).await;
            }

            PendingOperation::FetchSources {
                media,
                season,
                episode,
            } => {
                self.handle_fetch_sources(media, season, episode).await;
            }

            PendingOperation::ResolveStream(stream) => {
                self.handle_resolve_stream(stream).await;
            }
        }
    }

    /// Search both AniList and TMDB, merge results
    async fn handle_search(&mut self, query: &str) {
        // Search AniList and TMDB in parallel
        let (anilist_result, tmdb_result) = tokio::join!(
            self.anilist.search_anime(query),
            self.tmdb.search_all(query)
        );

        let mut results: Vec<Media> = Vec::new();

        // Add AniList results (anime)
        match anilist_result {
            Ok(anime_list) => {
                results.extend(anime_list.into_iter().map(Media::from));
            }
            Err(e) => {
                tracing::warn!("AniList search failed: {}", e);
            }
        }

        // Add TMDB results (movies and TV shows)
        match tmdb_result {
            Ok(tmdb_list) => {
                results.extend(tmdb_list);
            }
            Err(e) => {
                tracing::warn!("TMDB search failed: {}", e);
            }
        }

        if results.is_empty() {
            self.screen = Screen::Error(ErrorScreen::new(
                "No results found. Try a different search term.".to_string(),
                true,
            ));
        } else {
            // Sort results: prioritize by score (descending), then by year (descending)
            results.sort_by(|a, b| {
                let score_a = a.score.unwrap_or(0.0);
                let score_b = b.score.unwrap_or(0.0);
                score_b
                    .partial_cmp(&score_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            self.screen = Screen::Results(ResultsScreen::new(query.to_string(), results));
        }
    }

    /// Handle media selection based on type
    async fn handle_select_media(&mut self, media: Media) {
        match media.media_type {
            MediaType::Movie => {
                // Movies go directly to sources (no episode selection)
                self.pending = PendingOperation::FetchSources {
                    media,
                    season: 0,  // Not applicable for movies
                    episode: 0, // Not applicable for movies
                };
            }
            MediaType::TvShow => {
                // TV shows need season selection first
                self.pending = PendingOperation::FetchSeasons(media);
            }
            MediaType::Anime => {
                // Anime goes directly to episode selection (assume season 1)
                self.pending = PendingOperation::FetchEpisodes(media, None);
            }
        }
    }

    /// Fetch seasons for TV shows
    async fn handle_fetch_seasons(&mut self, media: Media) {
        let tmdb_id = match media.tmdb_id() {
            Some(id) => id,
            None => {
                self.screen = Screen::Error(ErrorScreen::new(
                    "Cannot fetch seasons: no TMDB ID available".to_string(),
                    false,
                ));
                return;
            }
        };

        match self.tmdb.get_tv_details(tmdb_id).await {
            Ok(seasons) => {
                if seasons.is_empty() {
                    self.screen = Screen::Error(ErrorScreen::new(
                        "No seasons found for this show".to_string(),
                        false,
                    ));
                } else if seasons.len() == 1 {
                    // Only one season, skip to episodes
                    let season = seasons.into_iter().next().unwrap();
                    self.screen = Screen::Episodes(EpisodesScreen::with_season(media, season));
                } else {
                    self.screen = Screen::Seasons(SeasonsScreen::new(media, seasons));
                }
            }
            Err(e) => {
                self.screen = Screen::Error(ErrorScreen::new(e.to_string(), true));
            }
        }
    }

    /// Fetch episodes for anime or TV show season
    async fn handle_fetch_episodes(&mut self, media: Media, season: Option<Season>) {
        match season {
            Some(s) => {
                self.screen = Screen::Episodes(EpisodesScreen::with_season(media, s));
            }
            None => {
                // Anime - use episodes from media directly
                self.screen = Screen::Episodes(EpisodesScreen::new(media));
            }
        }
    }

    /// Fetch sources from Torrentio
    async fn handle_fetch_sources(&mut self, media: Media, season: u32, episode: u32) {
        // Get IMDB ID based on source
        let imdb_id = match self.get_imdb_id(&media).await {
            Ok(id) => id,
            Err(e) => {
                self.screen = Screen::Error(ErrorScreen::new(e.to_string(), false));
                return;
            }
        };

        // Fetch streams based on media type
        let streams_result = match media.media_type {
            MediaType::Movie => self.torrentio.get_movie_streams(&imdb_id).await,
            MediaType::Anime | MediaType::TvShow => {
                self.torrentio.get_streams(&imdb_id, season, episode).await
            }
        };

        match streams_result {
            Ok(streams) => {
                if streams.is_empty() {
                    self.screen = Screen::Error(ErrorScreen::new(
                        "No sources found. Try a different title or episode.".to_string(),
                        false,
                    ));
                } else {
                    let title = media.display_title().to_string();
                    let ep_num = if media.media_type == MediaType::Movie {
                        0
                    } else {
                        episode
                    };
                    self.screen = Screen::Sources(SourcesScreen::new(title, ep_num, streams));
                }
            }
            Err(e) => {
                self.screen = Screen::Error(ErrorScreen::new(e.to_string(), true));
            }
        }
    }

    /// Get IMDB ID for a media item
    async fn get_imdb_id(&self, media: &Media) -> std::result::Result<String, crate::error::ApiError> {
        // If we already have IMDB ID, use it
        if let Some(imdb_id) = &media.imdb_id {
            return Ok(imdb_id.clone());
        }

        match &media.source {
            MediaSource::AniList { id, id_mal } => {
                // Use ARM server for anime
                self.mapping.anilist_to_imdb(*id, *id_mal).await
            }
            MediaSource::Tmdb { id } => {
                // Use TMDB external IDs
                match media.media_type {
                    MediaType::Movie => self.tmdb.get_movie_external_ids(*id).await,
                    MediaType::TvShow => self.tmdb.get_tv_external_ids(*id).await,
                    MediaType::Anime => {
                        // Shouldn't happen - anime comes from AniList
                        Err(crate::error::ApiError::MappingNotFound)
                    }
                }
            }
        }
    }

    /// Resolve and play stream
    async fn handle_resolve_stream(&mut self, stream: Stream) {
        let url = match &stream.url {
            Some(url) => url.clone(),
            None => {
                self.screen = Screen::Error(ErrorScreen::new(
                    "No URL available for this source".to_string(),
                    false,
                ));
                return;
            }
        };

        // Restore terminal before launching player
        disable_raw_mode().ok();
        execute!(io::stdout(), LeaveAlternateScreen).ok();

        match self.player.play(&url) {
            Ok(()) => {
                // Player finished, restore TUI
                enable_raw_mode().ok();
                execute!(io::stdout(), EnterAlternateScreen).ok();
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
