use thiserror::Error;

/// Application-wide result type
pub type Result<T> = anyhow::Result<T>;

/// API-specific errors with typed variants for matching
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ApiError {
    #[error("Anilist API error: {0}")]
    Anilist(String),

    #[error("TMDB API error: {0}")]
    Tmdb(String),

    #[error("Real-Debrid API error: {0}")]
    RealDebrid(String),

    #[error("Real-Debrid authentication failed. Please check your API key.")]
    RealDebridAuth,

    #[error("Torrentio error: {0}")]
    Torrentio(String),

    #[error("ID mapping error: {0}")]
    Mapping(String),

    #[error("Could not find IMDB ID mapping for this title.\n\nThis anime may not have an IMDB entry in the mapping database.\nTry searching with the Japanese or alternative title.")]
    MappingNotFound,

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Request timeout")]
    Timeout,
}

/// Configuration errors
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum ConfigError {
    #[error("Config file not found. Run 'miru init' to set up.")]
    NotFound,

    #[error("Invalid config file: {0}")]
    Invalid(String),

    #[error("Real-Debrid API key is required. Run 'miru init' to set up.")]
    MissingApiKey,

    #[error("Failed to save config: {0}")]
    SaveFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Player errors
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum PlayerError {
    #[error("Player '{0}' not found. Please install it or configure a different player.")]
    NotFound(String),

    #[error("Player exited with error: {0}")]
    ExitError(String),

    #[error("Failed to launch player: {0}")]
    LaunchFailed(String),
}
