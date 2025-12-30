mod anilist;
pub mod media;
mod mapping;
mod realdebrid;
mod tmdb;
mod torrentio;

pub use anilist::AnilistClient;
pub use mapping::MappingClient;
pub use media::{Episode, Media, MediaSource, MediaType, Season};
pub use realdebrid::RealDebridClient;
pub use tmdb::TmdbClient;
pub use torrentio::{Stream, TorrentioClient};
