use thiserror::Error;

/// Application-wide result type
pub type Result<T> = anyhow::Result<T>;

/// API-specific errors with typed variants for matching
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("TMDB API error: {0}")]
    Tmdb(String),

    #[error("Real-Debrid API error: {0}")]
    RealDebrid(String),

    #[error("Real-Debrid authentication failed. Please check your API key.")]
    RealDebridAuth,

    #[error("Torrentio error: {0}")]
    Torrentio(String),

    #[error("Could not find IMDB ID for this title.\n\nThis title may not have an IMDB entry.\nTry searching with an alternative title.")]
    MappingNotFound,

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
}

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file not found. Run 'miru init' to set up.")]
    NotFound,

    #[error("Invalid config file: {0}")]
    Invalid(String),

    #[error("Real-Debrid API key is required. Run 'miru init' to set up.")]
    #[allow(dead_code)]
    MissingApiKey,

    #[error("Failed to save config: {0}")]
    SaveFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Player errors
#[derive(Error, Debug)]
pub enum PlayerError {
    #[error("Player '{0}' not found. Please install it or configure a different player.")]
    NotFound(String),

    #[error("Player exited with error: {0}")]
    ExitError(String),

    #[error("Failed to launch player: {0}")]
    LaunchFailed(String),
}

/// Streaming errors (for direct torrent streaming)
#[derive(Error, Debug)]
pub enum StreamingError {
    #[error("Failed to initialize streaming session: {0}")]
    SessionInit(String),

    #[error("Failed to add torrent: {0}")]
    AddTorrent(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("No video file found: {0}")]
    NoVideoFile(String),

    #[error("Streaming error: {0}")]
    #[allow(dead_code)]
    Other(String),
}
