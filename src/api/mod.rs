pub mod media;
pub mod source_scoring;
mod realdebrid;
mod tmdb;
pub mod torrentio;

pub use media::{Episode, Media, MediaType, Season};
pub use realdebrid::RealDebridClient;
pub use source_scoring::{ScoringOptions, sort_streams_by_score, get_recommended_indices, pin_recommended_to_top, calculate_source_score};
pub use tmdb::TmdbClient;
pub use torrentio::{Stream, TorrentioClient};
