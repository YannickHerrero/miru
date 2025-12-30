mod loader;
mod schema;

pub use loader::{config_path, load_config, save_config};
pub use schema::{Config, PlayerConfig, TorrentioConfig};
