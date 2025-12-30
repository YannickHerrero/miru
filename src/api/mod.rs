mod anilist;
mod mapping;
mod realdebrid;
mod torrentio;

pub use anilist::{AnilistClient, Anime, Episode};
pub use mapping::MappingClient;
pub use realdebrid::RealDebridClient;
pub use torrentio::{Stream, TorrentioClient};
