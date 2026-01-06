use std::io::{self, Stdout};
use std::sync::Arc;
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
use tokio::sync::RwLock;

use crate::api::{Media, MediaType, Season, Stream, TmdbClient, TorrentioClient};
use crate::config::Config;
use crate::error::Result;
use crate::history::{WatchHistory, WatchedItem};
use crate::player::Player;
use crate::streaming::TorrentStreamer;
use crate::ui::components::Spinner;
use crate::ui::screens::{
    EpisodesAction, EpisodesScreen, ErrorAction, ErrorScreen, ResultsAction, ResultsScreen,
    SearchAction, SearchScreen, SeasonsAction, SeasonsScreen, SourcesAction, SourcesContext,
    SourcesScreen,
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
        show_uncached: bool,
    },
    RefetchSources {
        context: SourcesContext,
        show_uncached: bool,
    },
    ResolveStream(Stream),
    /// Start P2P streaming for a torrent
    StartP2PStream(Stream),
    /// Select an item from watch history
    SelectHistoryItem(WatchedItem),
}

/// Context for tracking what's currently being played
#[derive(Clone)]
struct PlaybackContext {
    media: Media,
    season: u32,
    episode: u32,
    episode_title: Option<String>,
}

/// Main TUI application
pub struct App {
    screen: Screen,
    pending: PendingOperation,
    should_quit: bool,
    // API clients
    tmdb: TmdbClient,
    torrentio: TorrentioClient,
    player: Player,
    // Theme
    theme: Theme,
    /// Whether to use direct P2P streaming (no Real-Debrid)
    #[allow(dead_code)]
    use_direct_streaming: bool,
    /// Torrent streamer for P2P playback (lazily initialized)
    torrent_streamer: Arc<RwLock<Option<TorrentStreamer>>>,
    /// Streaming HTTP port
    #[allow(dead_code)]
    streaming_port: u16,
    /// Watch history database
    history: Option<WatchHistory>,
    /// Current playback context (for recording history)
    playback_context: Option<PlaybackContext>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let use_direct_streaming = config.use_direct_streaming();

        // Create Torrentio client based on whether we have RD configured
        let torrentio = if use_direct_streaming {
            TorrentioClient::new_without_debrid(config.torrentio.clone())
        } else {
            TorrentioClient::new(config.torrentio.clone(), config.real_debrid.api_key.clone())
        };

        let tmdb = TmdbClient::new(config.tmdb.api_key.clone());
        let player = Player::new(config.player.clone());
        let streaming_port = config.streaming.http_port;

        // Open watch history database
        let history = WatchHistory::open().ok();

        // Load initial history
        let recent_history = history
            .as_ref()
            .map(|h| h.get_recent_media(10))
            .unwrap_or_default();

        Self {
            screen: Screen::Search(SearchScreen::new_with_history(recent_history)),
            pending: PendingOperation::None,
            should_quit: false,
            tmdb,
            torrentio,
            player,
            theme: Theme::default(),
            use_direct_streaming,
            torrent_streamer: Arc::new(RwLock::new(None)),
            streaming_port,
            history,
            playback_context: None,
        }
    }

    /// Get recent watch history items
    pub fn get_recent_history(&self) -> Vec<WatchedItem> {
        self.history
            .as_ref()
            .map(|h| h.get_recent_media(10))
            .unwrap_or_default()
    }

    /// Check if an episode is watched
    #[allow(dead_code)]
    pub fn is_episode_watched(
        &self,
        tmdb_id: i32,
        media_type: MediaType,
        season: u32,
        episode: u32,
    ) -> bool {
        self.history
            .as_ref()
            .map(|h| h.is_watched(tmdb_id, media_type, season, episode))
            .unwrap_or(false)
    }

    /// Get watched episode count for a season
    #[allow(dead_code)]
    pub fn watched_episode_count(&self, tmdb_id: i32, season: u32) -> u32 {
        self.history
            .as_ref()
            .map(|h| h.watched_episode_count(tmdb_id, season))
            .unwrap_or(0)
    }

    /// Record the current playback to history
    fn record_playback(&self) {
        if let (Some(history), Some(ctx)) = (&self.history, &self.playback_context) {
            let _ = history.mark_watched(
                ctx.media.tmdb_id(),
                ctx.media.media_type,
                &ctx.media.title,
                ctx.season,
                ctx.episode,
                ctx.episode_title.as_deref(),
                ctx.media.cover_image.as_deref(),
            );
        }
    }

    /// Create a new search screen with current history
    fn new_search_screen(&self) -> SearchScreen {
        SearchScreen::new_with_history(self.get_recent_history())
    }

    /// Get watched episodes for a specific season of a show
    fn get_watched_episodes_for_season(
        &self,
        tmdb_id: i32,
        season_number: u32,
    ) -> std::collections::HashSet<u32> {
        self.history
            .as_ref()
            .map(|h| {
                // Query all watched episodes for this media and season
                h.get_watched_episodes(tmdb_id, season_number)
            })
            .unwrap_or_default()
    }

    /// Get watched episode counts for all seasons of a show
    fn get_watched_counts_by_season(
        &self,
        tmdb_id: i32,
        seasons: &[Season],
    ) -> std::collections::HashMap<u32, u32> {
        self.history
            .as_ref()
            .map(|h| {
                seasons
                    .iter()
                    .map(|s| (s.number, h.watched_episode_count(tmdb_id, s.number)))
                    .collect()
            })
            .unwrap_or_default()
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
                if let Some(action) = screen.handle_key(key) {
                    match action {
                        SearchAction::Search(query) => {
                            self.pending = PendingOperation::Search(query);
                            self.screen = Screen::Loading(Spinner::new("Searching..."));
                        }
                        SearchAction::SelectHistory(item) => {
                            // Fetch the media from TMDB and navigate appropriately
                            self.pending = PendingOperation::SelectHistoryItem(item);
                            self.screen = Screen::Loading(Spinner::new("Loading..."));
                        }
                    }
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
                            let query = screen.query.clone();
                            let history = self.get_recent_history();
                            self.screen = Screen::Search(SearchScreen::with_query_and_history(
                                &query, history,
                            ));
                        }
                        ResultsAction::Search => {
                            self.screen = Screen::Search(self.new_search_screen());
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
                            self.screen = Screen::Search(self.new_search_screen());
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
                                show_uncached: false,
                            };
                            self.screen = Screen::Loading(Spinner::new("Fetching sources..."));
                        }
                        EpisodesAction::ToggleWatched(episode) => {
                            let season_num = screen.season_number();
                            let tmdb_id = screen.media.tmdb_id();
                            let is_currently_watched = screen.is_watched(episode.number);

                            // Toggle in database
                            if let Some(history) = &self.history {
                                if is_currently_watched {
                                    let _ = history.mark_unwatched(
                                        tmdb_id,
                                        screen.media.media_type,
                                        season_num,
                                        episode.number,
                                    );
                                } else {
                                    let _ = history.mark_watched(
                                        tmdb_id,
                                        screen.media.media_type,
                                        &screen.media.title,
                                        season_num,
                                        episode.number,
                                        Some(&episode.title),
                                        screen.media.cover_image.as_deref(),
                                    );
                                }
                            }

                            // Toggle locally in the screen
                            screen.toggle_watched(episode.number);
                        }
                        EpisodesAction::Back => {
                            self.screen = Screen::Search(self.new_search_screen());
                        }
                    }
                }
            }
            Screen::Sources(screen) => {
                if let Some(action) = screen.handle_key(key) {
                    match action {
                        SourcesAction::Select(stream) => {
                            // Set playback context for history tracking
                            self.playback_context = Some(PlaybackContext {
                                media: screen.context.media.clone(),
                                season: screen.context.season,
                                episode: screen.context.episode,
                                episode_title: None, // Could be enhanced to get episode title
                            });
                            self.pending = PendingOperation::ResolveStream(stream);
                            self.screen = Screen::Loading(Spinner::new("Resolving stream..."));
                        }
                        SourcesAction::Back => {
                            self.screen = Screen::Search(self.new_search_screen());
                        }
                        SourcesAction::ToggleUncached => {
                            let new_show_uncached = !screen.show_uncached;
                            self.pending = PendingOperation::RefetchSources {
                                context: screen.context.clone(),
                                show_uncached: new_show_uncached,
                            };
                            let msg = if new_show_uncached {
                                "Fetching all sources..."
                            } else {
                                "Fetching cached sources..."
                            };
                            self.screen = Screen::Loading(Spinner::new(msg));
                        }
                    }
                }
            }
            Screen::Loading(_) => {
                // Allow cancelling with Esc
                if key.code == KeyCode::Esc {
                    self.pending = PendingOperation::None;
                    self.screen = Screen::Search(self.new_search_screen());
                }
            }
            Screen::Error(screen) => {
                if let Some(action) = screen.handle_key(key) {
                    match action {
                        ErrorAction::Retry => {
                            self.screen = Screen::Search(self.new_search_screen());
                        }
                        ErrorAction::Back => {
                            self.screen = Screen::Search(self.new_search_screen());
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
                show_uncached,
            } => {
                self.handle_fetch_sources(media, season, episode, show_uncached)
                    .await;
            }

            PendingOperation::RefetchSources {
                context,
                show_uncached,
            } => {
                self.handle_refetch_sources(context, show_uncached).await;
            }

            PendingOperation::ResolveStream(stream) => {
                self.handle_resolve_stream(stream).await;
            }

            PendingOperation::StartP2PStream(stream) => {
                self.handle_start_p2p_stream(stream).await;
            }

            PendingOperation::SelectHistoryItem(item) => {
                self.handle_select_history_item(item).await;
            }
        }
    }

    /// Handle selecting an item from watch history
    async fn handle_select_history_item(&mut self, item: WatchedItem) {
        // Fetch media details from TMDB
        let result = match item.media_type {
            MediaType::Movie => self.tmdb.get_movie_details(item.tmdb_id).await,
            MediaType::TvShow => self.tmdb.get_tv_show_details(item.tmdb_id).await,
        };

        match result {
            Ok(media) => {
                // For movies, go directly to sources
                // For TV shows, go to the season/episode selection
                match media.media_type {
                    MediaType::Movie => {
                        self.pending = PendingOperation::FetchSources {
                            media,
                            season: 0,
                            episode: 0,
                            show_uncached: false,
                        };
                    }
                    MediaType::TvShow => {
                        // If we have season/episode from history, fetch that season's episodes
                        if item.season > 0 {
                            let season = Season {
                                number: item.season,
                                episode_count: 0, // Will be populated from the episodes
                            };
                            self.pending = PendingOperation::FetchEpisodes(media, Some(season));
                        } else {
                            self.pending = PendingOperation::FetchSeasons(media);
                        }
                    }
                }
            }
            Err(e) => {
                self.screen = Screen::Error(ErrorScreen::new(
                    format!("Failed to load media details: {}", e),
                    true,
                ));
            }
        }
    }

    /// Search TMDB for movies and TV shows
    async fn handle_search(&mut self, query: &str) {
        match self.tmdb.search_all(query).await {
            Ok(mut results) => {
                if results.is_empty() {
                    self.screen = Screen::Error(ErrorScreen::new(
                        "No results found. Try a different search term.".to_string(),
                        true,
                    ));
                } else {
                    // Sort results by score (descending)
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
            Err(e) => {
                self.screen = Screen::Error(ErrorScreen::new(e.to_string(), true));
            }
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
                    show_uncached: false,
                };
            }
            MediaType::TvShow => {
                // TV shows need season selection first
                self.pending = PendingOperation::FetchSeasons(media);
            }
        }
    }

    /// Fetch seasons for TV shows
    async fn handle_fetch_seasons(&mut self, media: Media) {
        let tmdb_id = media.tmdb_id();

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
                    let watched =
                        self.get_watched_episodes_for_season(media.tmdb_id(), season.number);
                    let mut screen = EpisodesScreen::with_season(media, season);
                    screen.set_watched_episodes(watched);
                    self.screen = Screen::Episodes(screen);
                } else {
                    let watched_counts =
                        self.get_watched_counts_by_season(media.tmdb_id(), &seasons);
                    let mut screen = SeasonsScreen::new(media, seasons);
                    screen.set_watched_counts(watched_counts);
                    self.screen = Screen::Seasons(screen);
                }
            }
            Err(e) => {
                self.screen = Screen::Error(ErrorScreen::new(e.to_string(), true));
            }
        }
    }

    /// Fetch episodes for TV show season
    async fn handle_fetch_episodes(&mut self, media: Media, season: Option<Season>) {
        match season {
            Some(s) => {
                let watched = self.get_watched_episodes_for_season(media.tmdb_id(), s.number);
                let mut screen = EpisodesScreen::with_season(media, s);
                screen.set_watched_episodes(watched);
                self.screen = Screen::Episodes(screen);
            }
            None => {
                // For anime without seasons, use season 1
                let watched = self.get_watched_episodes_for_season(media.tmdb_id(), 1);
                let mut screen = EpisodesScreen::new(media);
                screen.set_watched_episodes(watched);
                self.screen = Screen::Episodes(screen);
            }
        }
    }

    /// Fetch sources from Torrentio
    async fn handle_fetch_sources(
        &mut self,
        media: Media,
        season: u32,
        episode: u32,
        show_uncached: bool,
    ) {
        // Get IMDB ID based on source
        let imdb_id = match self.get_imdb_id(&media).await {
            Ok(id) => id,
            Err(e) => {
                self.screen = Screen::Error(ErrorScreen::new(e.to_string(), false));
                return;
            }
        };

        // Create context for potential re-fetching
        let context = SourcesContext {
            media: media.clone(),
            season,
            episode,
            imdb_id: imdb_id.clone(),
        };

        // Fetch streams based on media type
        let streams_result = match media.media_type {
            MediaType::Movie => {
                self.torrentio
                    .get_movie_streams(&imdb_id, show_uncached)
                    .await
            }
            MediaType::TvShow => {
                self.torrentio
                    .get_streams(&imdb_id, season, episode, show_uncached)
                    .await
            }
        };

        match streams_result {
            Ok(streams) => {
                // Always show sources screen, even if empty
                let title = media.display_title().to_string();
                let ep_num = if media.media_type == MediaType::Movie {
                    0
                } else {
                    episode
                };
                self.screen = Screen::Sources(SourcesScreen::new(
                    title,
                    ep_num,
                    streams,
                    context,
                    show_uncached,
                ));
            }
            Err(e) => {
                self.screen = Screen::Error(ErrorScreen::new(e.to_string(), true));
            }
        }
    }

    /// Re-fetch sources with different uncached setting
    async fn handle_refetch_sources(&mut self, context: SourcesContext, show_uncached: bool) {
        // Fetch streams based on media type
        let streams_result = match context.media.media_type {
            MediaType::Movie => {
                self.torrentio
                    .get_movie_streams(&context.imdb_id, show_uncached)
                    .await
            }
            MediaType::TvShow => {
                self.torrentio
                    .get_streams(
                        &context.imdb_id,
                        context.season,
                        context.episode,
                        show_uncached,
                    )
                    .await
            }
        };

        match streams_result {
            Ok(streams) => {
                let title = context.media.display_title().to_string();
                let ep_num = if context.media.media_type == MediaType::Movie {
                    0
                } else {
                    context.episode
                };
                self.screen = Screen::Sources(SourcesScreen::new(
                    title,
                    ep_num,
                    streams,
                    context,
                    show_uncached,
                ));
            }
            Err(e) => {
                self.screen = Screen::Error(ErrorScreen::new(e.to_string(), true));
            }
        }
    }

    /// Get IMDB ID for a media item
    async fn get_imdb_id(
        &self,
        media: &Media,
    ) -> std::result::Result<String, crate::error::ApiError> {
        // If we already have IMDB ID, use it
        if let Some(imdb_id) = &media.imdb_id {
            return Ok(imdb_id.clone());
        }

        let id = media.tmdb_id();
        match media.media_type {
            MediaType::Movie => self.tmdb.get_movie_external_ids(id).await,
            MediaType::TvShow => self.tmdb.get_tv_external_ids(id).await,
        }
    }

    /// Resolve and play stream
    async fn handle_resolve_stream(&mut self, stream: Stream) {
        // Check if we have a direct URL (Real-Debrid) or need P2P streaming
        if let Some(url) = &stream.url {
            // Real-Debrid: we have a direct HTTP URL
            self.play_url(url);
        } else if stream.info_hash.is_some() {
            // P2P streaming: need to use TorrentStreamer
            self.pending = PendingOperation::StartP2PStream(stream);
            self.screen = Screen::Loading(Spinner::new("Starting P2P stream..."));
        } else {
            self.screen = Screen::Error(ErrorScreen::new(
                "No URL or torrent hash available for this source".to_string(),
                false,
            ));
        }
    }

    /// Start P2P streaming for a torrent
    async fn handle_start_p2p_stream(&mut self, stream: Stream) {
        // Get the magnet link
        let magnet = match stream.magnet_link() {
            Some(m) => m,
            None => {
                self.screen = Screen::Error(ErrorScreen::new(
                    "No torrent hash available for P2P streaming".to_string(),
                    false,
                ));
                return;
            }
        };

        // Ensure torrent streamer is initialized
        {
            let mut streamer_guard = self.torrent_streamer.write().await;
            if streamer_guard.is_none() {
                match TorrentStreamer::new().await {
                    Ok(streamer) => {
                        *streamer_guard = Some(streamer);
                        tracing::info!("Torrent streamer initialized");
                    }
                    Err(e) => {
                        self.screen = Screen::Error(ErrorScreen::new(
                            format!("Failed to initialize torrent streaming: {}", e),
                            false,
                        ));
                        return;
                    }
                }
            }
        }

        // Start streaming the torrent
        let streamer_guard = self.torrent_streamer.read().await;
        let streamer = streamer_guard.as_ref().unwrap();

        match streamer.stream_magnet(&magnet).await {
            Ok(handle) => {
                tracing::info!("Streaming: {} at {}", handle.file_name, handle.stream_url);

                // Wait for buffering before starting playback
                self.screen = Screen::Loading(Spinner::new("Buffering..."));

                // Poll for ready state
                let mut ready = false;
                for _ in 0..60 {
                    // 30 second timeout
                    if let Some(progress) = streamer.get_progress().await {
                        if progress.ready_to_play {
                            ready = true;
                            break;
                        }
                        tracing::debug!(
                            "Buffering: {:.1}% ({} peers)",
                            progress.progress_percent,
                            progress.peers
                        );
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }

                drop(streamer_guard);

                if ready {
                    self.play_url(&handle.stream_url);
                } else {
                    self.screen = Screen::Error(ErrorScreen::new(
                        "Buffering timeout - not enough data to start playback.\n\nThis torrent may have few seeders.".to_string(),
                        false,
                    ));
                }
            }
            Err(e) => {
                self.screen = Screen::Error(ErrorScreen::new(
                    format!("Failed to start stream: {}", e),
                    false,
                ));
            }
        }
    }

    /// Play a URL with the configured player
    fn play_url(&mut self, url: &str) {
        // Record to watch history when playback starts
        self.record_playback();

        // Restore terminal before launching player
        disable_raw_mode().ok();
        execute!(io::stdout(), LeaveAlternateScreen).ok();

        match self.player.play(url) {
            Ok(()) => {
                // Player finished, restore TUI
                enable_raw_mode().ok();
                execute!(io::stdout(), EnterAlternateScreen).ok();
                // Return to search screen with updated history
                self.screen =
                    Screen::Search(SearchScreen::new_with_history(self.get_recent_history()));
            }
            Err(e) => {
                // Restore TUI even on error
                enable_raw_mode().ok();
                execute!(io::stdout(), EnterAlternateScreen).ok();
                self.screen = Screen::Error(ErrorScreen::new(e.to_string(), false));
            }
        }

        // Clear playback context
        self.playback_context = None;
    }

    /// Cleanup torrent streamer on shutdown
    #[allow(dead_code)]
    pub async fn cleanup(&self) {
        let streamer_guard = self.torrent_streamer.read().await;
        if let Some(streamer) = streamer_guard.as_ref() {
            streamer.cleanup().await;
        }
    }
}
