//! Direct torrent streaming module using librqbit.
//!
//! This module provides P2P torrent streaming capabilities, allowing playback
//! of torrents without requiring a debrid service like Real-Debrid.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, Api, ManagedTorrent, Session, SessionOptions,
};
use tokio::sync::RwLock;

use crate::error::StreamingError;

/// Default port for the librqbit HTTP API  
const DEFAULT_HTTP_PORT: u16 = 3131;

/// Video file extensions to look for in torrents
const VIDEO_EXTENSIONS: &[&str] = &[
    "mkv", "mp4", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "ts", "m2ts",
];

/// Torrent streaming manager using librqbit
pub struct TorrentStreamer {
    session: Arc<Session>,
    api: Api,
    http_port: u16,
    /// Currently active torrent handle
    active_torrent: RwLock<Option<ActiveTorrent>>,
}

/// Information about an active torrent stream
struct ActiveTorrent {
    #[allow(dead_code)]
    handle: Arc<ManagedTorrent>,
    #[allow(dead_code)]
    file_index: usize,
    torrent_id: usize,
}

/// Result of starting a stream
pub struct StreamHandle {
    /// HTTP URL to stream from (e.g., http://127.0.0.1:3131/torrents/0/stream/0)
    pub stream_url: String,
    /// File name being streamed
    pub file_name: String,
}

/// Progress information for a streaming torrent
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StreamProgress {
    /// Download progress as percentage (0.0 - 100.0)
    pub progress_percent: f64,
    /// Downloaded bytes
    pub downloaded_bytes: u64,
    /// Total bytes
    pub total_bytes: u64,
    /// Current download speed in bytes/sec
    pub download_speed: u64,
    /// Number of connected peers
    pub peers: usize,
    /// Whether enough data is buffered for playback
    pub ready_to_play: bool,
}

impl TorrentStreamer {
    /// Create a new torrent streamer with a temporary download directory
    pub async fn new() -> Result<Self, StreamingError> {
        let temp_dir = std::env::temp_dir().join("miru-torrents");
        Self::with_download_dir(temp_dir, DEFAULT_HTTP_PORT).await
    }

    /// Create a new torrent streamer with a specific download directory and port
    pub async fn with_download_dir(
        download_dir: PathBuf,
        http_port: u16,
    ) -> Result<Self, StreamingError> {
        // Create download directory if it doesn't exist
        if !download_dir.exists() {
            std::fs::create_dir_all(&download_dir).map_err(|e| {
                StreamingError::SessionInit(format!("Failed to create download dir: {}", e))
            })?;
        }

        // Use default options - librqbit handles all the complexity
        let opts = SessionOptions {
            disable_dht: false,
            disable_dht_persistence: true,
            enable_upnp_port_forwarding: false,
            ..Default::default()
        };

        let session = Session::new_with_opts(download_dir, opts)
            .await
            .map_err(|e| StreamingError::SessionInit(e.to_string()))?;

        let api = Api::new(session.clone(), None);

        Ok(Self {
            session,
            api,
            http_port,
            active_torrent: RwLock::new(None),
        })
    }

    /// Start streaming a magnet link
    ///
    /// Returns a StreamHandle with the HTTP URL for playback
    pub async fn stream_magnet(&self, magnet: &str) -> Result<StreamHandle, StreamingError> {
        tracing::info!(
            "Starting torrent stream for magnet: {}...",
            &magnet[..magnet.len().min(60)]
        );

        // Clean up any existing torrent first
        self.cleanup().await;

        // Add the torrent
        let add_torrent = AddTorrent::from_url(magnet);
        let opts = AddTorrentOptions {
            overwrite: true,
            ..Default::default()
        };

        let response = self
            .session
            .add_torrent(add_torrent, Some(opts))
            .await
            .map_err(|e| StreamingError::AddTorrent(e.to_string()))?;

        let (torrent_id, handle) = match response {
            AddTorrentResponse::Added(id, handle) => (id, handle),
            AddTorrentResponse::AlreadyManaged(id, handle) => (id, handle),
            AddTorrentResponse::ListOnly(_) => {
                return Err(StreamingError::AddTorrent(
                    "Torrent was added in list-only mode".to_string(),
                ));
            }
        };

        // Wait for metadata to be available
        let handle = self.wait_for_metadata(handle).await?;

        // Find the largest video file using the API
        let (file_index, file_name) = self.find_video_file(torrent_id).await?;

        tracing::info!("Streaming file: {} (index {})", file_name, file_index);

        // Store the active torrent
        {
            let mut active = self.active_torrent.write().await;
            *active = Some(ActiveTorrent {
                handle: handle.clone(),
                file_index,
                torrent_id,
            });
        }

        // Build the stream URL
        // librqbit API: /torrents/{id}/stream/{file_idx}
        let stream_url = format!(
            "http://127.0.0.1:{}/torrents/{}/stream/{}",
            self.http_port, torrent_id, file_index
        );

        Ok(StreamHandle {
            stream_url,
            file_name,
        })
    }

    /// Wait for torrent metadata to be available
    async fn wait_for_metadata(
        &self,
        handle: Arc<ManagedTorrent>,
    ) -> Result<Arc<ManagedTorrent>, StreamingError> {
        // Poll for metadata availability
        let timeout = Duration::from_secs(60);
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > timeout {
                return Err(StreamingError::Timeout(
                    "Timeout waiting for torrent metadata".to_string(),
                ));
            }

            // Check if we have file info available via stats
            let stats = handle.stats();
            if stats.total_bytes > 0 {
                return Ok(handle);
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Find the best video file in the torrent (largest video file)
    async fn find_video_file(&self, torrent_id: usize) -> Result<(usize, String), StreamingError> {
        // Use the API to get torrent details with file list
        let details = self
            .api
            .api_torrent_details(torrent_id.into())
            .map_err(|e| {
                StreamingError::NoVideoFile(format!("Failed to get torrent details: {}", e))
            })?;

        let files = details
            .files
            .ok_or_else(|| StreamingError::NoVideoFile("No files in torrent".to_string()))?;

        let mut best_match: Option<(usize, String, u64)> = None;

        for (idx, file) in files.iter().enumerate() {
            let filename = file.name.clone();
            let extension = filename.rsplit('.').next().unwrap_or("").to_lowercase();

            if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
                let size = file.length;
                if best_match.as_ref().map_or(true, |(_, _, s)| size > *s) {
                    best_match = Some((idx, filename, size));
                }
            }
        }

        best_match.map(|(idx, name, _)| (idx, name)).ok_or_else(|| {
            StreamingError::NoVideoFile("No video files found in torrent".to_string())
        })
    }

    /// Get current streaming progress
    pub async fn get_progress(&self) -> Option<StreamProgress> {
        let active = self.active_torrent.read().await;
        let active = active.as_ref()?;

        let details = self
            .api
            .api_torrent_details(active.torrent_id.into())
            .ok()?;
        let stats = details.stats?;

        let total_bytes = stats.total_bytes;
        let downloaded_bytes = stats.progress_bytes;
        let progress_percent = if total_bytes > 0 {
            (downloaded_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        // Get live stats for speed and peers
        let (download_speed, peers) = stats.live.as_ref().map_or((0, 0), |l| {
            let speed = (l.download_speed.mbps * 125_000.0) as u64; // mbps to bytes/sec
            let peer_count = l.snapshot.peer_stats.live;
            (speed, peer_count)
        });

        Some(StreamProgress {
            progress_percent,
            downloaded_bytes,
            total_bytes,
            download_speed,
            peers,
            // Consider ready to play when we have at least 2% or 5MB
            ready_to_play: progress_percent >= 2.0 || downloaded_bytes >= 5 * 1024 * 1024,
        })
    }

    /// Clean up the current torrent
    pub async fn cleanup(&self) {
        let mut active = self.active_torrent.write().await;
        if let Some(torrent) = active.take() {
            // Delete the torrent and its files
            let _ = self.session.delete(torrent.torrent_id.into(), true).await;
        }
    }

    /// Stop the streaming session
    #[allow(dead_code)]
    pub async fn stop(&self) {
        self.cleanup().await;
        self.session.stop().await;
    }

    /// Get the HTTP API port
    #[allow(dead_code)]
    pub fn http_port(&self) -> u16 {
        self.http_port
    }
}

impl Drop for TorrentStreamer {
    fn drop(&mut self) {
        // Note: async cleanup is handled by stop() method
        // This is just a safety fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_extensions() {
        assert!(VIDEO_EXTENSIONS.contains(&"mkv"));
        assert!(VIDEO_EXTENSIONS.contains(&"mp4"));
        assert!(!VIDEO_EXTENSIONS.contains(&"txt"));
    }
}
