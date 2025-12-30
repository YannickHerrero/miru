use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;

use crate::config::TorrentioConfig;
use crate::error::ApiError;

const TORRENTIO_URL: &str = "https://torrentio.strem.fun";

lazy_static! {
    // Match patterns like "👤 150" for seeders
    static ref SEEDERS_RE: Regex = Regex::new(r"👤\s*(\d+)").unwrap();
    // Match patterns like "💾 1.2 GB" or "💾 800 MB" for size
    static ref SIZE_RE: Regex = Regex::new(r"💾\s*([\d.]+)\s*(GB|MB|TB)").unwrap();
    // Match quality patterns like "1080p", "720p", "480p", "2160p", "4K"
    static ref QUALITY_RE: Regex = Regex::new(r"\b(2160p|4K|1080p|720p|480p|360p)\b").unwrap();
}

/// Torrentio addon client
pub struct TorrentioClient {
    client: Client,
    config: TorrentioConfig,
    rd_api_key: String,
}

impl TorrentioClient {
    pub fn new(config: TorrentioConfig, rd_api_key: String) -> Self {
        Self {
            client: Client::new(),
            config,
            rd_api_key,
        }
    }

    /// Build the config string for Torrentio URL
    fn build_config_string(&self) -> String {
        let providers = self.config.providers.join(",");
        // debridoptions=nodownloadlinks ensures only cached/instant streams are returned
        // This means all URLs are direct RD links that can be played immediately
        format!(
            "providers={}|sort=qualitysize|qualityfilter=scr,cam|debridoptions=nodownloadlinks|realdebrid={}",
            providers, self.rd_api_key
        )
    }

    /// Get streams for a series episode
    pub async fn get_streams(
        &self,
        imdb_id: &str,
        season: u32,
        episode: u32,
    ) -> Result<Vec<Stream>, ApiError> {
        let config_str = self.build_config_string();
        let url = format!(
            "{}/{}/stream/series/{}:{}:{}.json",
            TORRENTIO_URL, config_str, imdb_id, season, episode
        );

        tracing::debug!("Fetching streams from: {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ApiError::Torrentio(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let data: TorrentioResponse = response.json().await.map_err(|e| {
            ApiError::Torrentio(format!("Failed to parse response: {}", e))
        })?;

        Ok(data
            .streams
            .into_iter()
            .map(Stream::from)
            .collect())
    }

    /// Get streams for a movie
    #[allow(dead_code)]
    pub async fn get_movie_streams(&self, imdb_id: &str) -> Result<Vec<Stream>, ApiError> {
        let config_str = self.build_config_string();
        let url = format!(
            "{}/{}/stream/movie/{}.json",
            TORRENTIO_URL, config_str, imdb_id
        );

        tracing::debug!("Fetching movie streams from: {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ApiError::Torrentio(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let data: TorrentioResponse = response.json().await.map_err(|e| {
            ApiError::Torrentio(format!("Failed to parse response: {}", e))
        })?;

        Ok(data
            .streams
            .into_iter()
            .map(Stream::from)
            .collect())
    }
}

#[derive(Debug, Deserialize)]
struct TorrentioResponse {
    streams: Vec<StreamResponse>,
}

#[derive(Debug, Deserialize)]
struct StreamResponse {
    name: String,
    title: String,
    url: Option<String>,
    #[serde(rename = "infoHash")]
    info_hash: Option<String>,
    #[allow(dead_code)]
    #[serde(rename = "behaviorHints")]
    behavior_hints: Option<BehaviorHints>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BehaviorHints {
    #[serde(rename = "bingeGroup")]
    binge_group: Option<String>,
}

/// Parsed stream data
#[derive(Debug, Clone)]
pub struct Stream {
    /// Provider name (e.g., "nyaasi", "1337x")
    pub provider: String,
    /// Full title with metadata
    #[allow(dead_code)]
    pub title: String,
    /// Quality (e.g., "1080p", "720p")
    pub quality: Option<String>,
    /// File size as string (e.g., "1.2 GB")
    pub size: Option<String>,
    /// Number of seeders
    pub seeders: Option<u32>,
    /// Whether this is cached on Real-Debrid
    pub is_cached: bool,
    /// Stream URL (direct or magnet)
    pub url: Option<String>,
    /// Info hash for magnet links
    #[allow(dead_code)]
    pub info_hash: Option<String>,
}

impl Stream {
    /// Get a short display string for the stream
    #[allow(dead_code)]
    pub fn display(&self) -> String {
        let mut parts = vec![format!("[{}]", self.provider)];

        if let Some(q) = &self.quality {
            parts.push(q.clone());
        }

        if let Some(s) = &self.size {
            parts.push(s.clone());
        }

        if let Some(seeders) = self.seeders {
            parts.push(format!("👤{}", seeders));
        }

        parts.join(" ")
    }

    /// Get cache status indicator
    pub fn cache_indicator(&self) -> &str {
        if self.is_cached {
            "🟢"
        } else {
            "🟡"
        }
    }
}

impl From<StreamResponse> for Stream {
    fn from(resp: StreamResponse) -> Self {
        // Parse provider from name (e.g., "[RD+] nyaasi" -> "nyaasi")
        let provider = resp
            .name
            .split(']')
            .last()
            .map(|s| s.trim())
            .unwrap_or(&resp.name)
            .to_string();

        // Check if cached (name contains "[RD+]" or "[RD download]")
        let is_cached = resp.name.contains("[RD+]") || resp.name.contains("⚡");

        // Parse quality from title
        let quality = QUALITY_RE
            .find(&resp.title)
            .map(|m| m.as_str().to_string());

        // Parse size from title
        let size = SIZE_RE.captures(&resp.title).map(|caps| {
            format!("{} {}", &caps[1], &caps[2])
        });

        // Parse seeders from title
        let seeders = SEEDERS_RE
            .captures(&resp.title)
            .and_then(|caps| caps[1].parse().ok());

        Self {
            provider,
            title: resp.title,
            quality,
            size,
            seeders,
            is_cached,
            url: resp.url,
            info_hash: resp.info_hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_stream_title() {
        let resp = StreamResponse {
            name: "[RD+] nyaasi".to_string(),
            title: "Frieren S01E01 1080p WEB x264\n👤 150 💾 1.2 GB".to_string(),
            url: Some("https://example.com".to_string()),
            info_hash: None,
            behavior_hints: None,
        };

        let stream = Stream::from(resp);

        assert_eq!(stream.provider, "nyaasi");
        assert!(stream.is_cached);
        assert_eq!(stream.quality, Some("1080p".to_string()));
        assert_eq!(stream.size, Some("1.2 GB".to_string()));
        assert_eq!(stream.seeders, Some(150));
    }

    #[test]
    fn test_parse_stream_not_cached() {
        let resp = StreamResponse {
            name: "[RD download] 1337x".to_string(),
            title: "Some Anime 720p\n👤 50 💾 800 MB".to_string(),
            url: None,
            info_hash: Some("abc123".to_string()),
            behavior_hints: None,
        };

        let stream = Stream::from(resp);

        assert_eq!(stream.provider, "1337x");
        assert!(!stream.is_cached);
        assert_eq!(stream.quality, Some("720p".to_string()));
        assert_eq!(stream.size, Some("800 MB".to_string()));
        assert_eq!(stream.seeders, Some(50));
    }
}
