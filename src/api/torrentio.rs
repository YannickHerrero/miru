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

        let mut streams: Vec<Stream> = data.streams.into_iter().map(Stream::from).collect();

        // Sort by quality (descending), then by size (ascending)
        streams.sort_by(|a, b| {
            match b.quality_rank().cmp(&a.quality_rank()) {
                std::cmp::Ordering::Equal => a.size_bytes.cmp(&b.size_bytes),
                other => other,
            }
        });

        Ok(streams)
    }

    /// Get streams for a movie
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

        let mut streams: Vec<Stream> = data.streams.into_iter().map(Stream::from).collect();

        // Sort by quality (descending), then by size (ascending)
        streams.sort_by(|a, b| {
            match b.quality_rank().cmp(&a.quality_rank()) {
                std::cmp::Ordering::Equal => a.size_bytes.cmp(&b.size_bytes),
                other => other,
            }
        });

        Ok(streams)
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
    /// File size in bytes (for sorting)
    pub size_bytes: u64,
    /// Number of seeders
    pub seeders: Option<u32>,
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

    /// Get quality rank for sorting (higher is better)
    pub fn quality_rank(&self) -> u8 {
        match self.quality.as_deref() {
            Some("2160p") | Some("4K") => 4,
            Some("1080p") => 3,
            Some("720p") => 2,
            Some("480p") | Some("360p") => 1,
            _ => 0,
        }
    }
}

/// Parse size string like "1.2 GB" or "800 MB" into bytes
fn parse_size_to_bytes(size_str: &str) -> u64 {
    let parts: Vec<&str> = size_str.split_whitespace().collect();
    if parts.len() != 2 {
        return u64::MAX;
    }

    let value: f64 = match parts[0].parse() {
        Ok(v) => v,
        Err(_) => return u64::MAX,
    };

    let multiplier: u64 = match parts[1].to_uppercase().as_str() {
        "TB" => 1024 * 1024 * 1024 * 1024,
        "GB" => 1024 * 1024 * 1024,
        "MB" => 1024 * 1024,
        "KB" => 1024,
        _ => return u64::MAX,
    };

    (value * multiplier as f64) as u64
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

        // Parse quality from title
        let quality = QUALITY_RE
            .find(&resp.title)
            .map(|m| m.as_str().to_string());

        // Parse size from title
        let size = SIZE_RE.captures(&resp.title).map(|caps| {
            format!("{} {}", &caps[1], &caps[2])
        });

        // Parse size into bytes for sorting
        let size_bytes = size
            .as_ref()
            .map(|s| parse_size_to_bytes(s))
            .unwrap_or(u64::MAX);

        // Parse seeders from title
        let seeders = SEEDERS_RE
            .captures(&resp.title)
            .and_then(|caps| caps[1].parse().ok());

        Self {
            provider,
            title: resp.title,
            quality,
            size,
            size_bytes,
            seeders,
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
        assert_eq!(stream.quality, Some("1080p".to_string()));
        assert_eq!(stream.size, Some("1.2 GB".to_string()));
        assert_eq!(stream.seeders, Some(150));
        assert_eq!(stream.quality_rank(), 3); // 1080p = rank 3
    }

    #[test]
    fn test_parse_stream_720p() {
        let resp = StreamResponse {
            name: "[RD+] 1337x".to_string(),
            title: "Some Anime 720p\n👤 50 💾 800 MB".to_string(),
            url: None,
            info_hash: Some("abc123".to_string()),
            behavior_hints: None,
        };

        let stream = Stream::from(resp);

        assert_eq!(stream.provider, "1337x");
        assert_eq!(stream.quality, Some("720p".to_string()));
        assert_eq!(stream.size, Some("800 MB".to_string()));
        assert_eq!(stream.seeders, Some(50));
        assert_eq!(stream.quality_rank(), 2); // 720p = rank 2
    }

    #[test]
    fn test_parse_size_to_bytes() {
        assert_eq!(parse_size_to_bytes("1 GB"), 1024 * 1024 * 1024);
        assert_eq!(parse_size_to_bytes("800 MB"), 800 * 1024 * 1024);
        assert_eq!(parse_size_to_bytes("1.5 GB"), (1.5 * 1024.0 * 1024.0 * 1024.0) as u64);
        assert_eq!(parse_size_to_bytes("invalid"), u64::MAX);
    }

    #[test]
    fn test_quality_rank() {
        let make_stream = |quality: Option<&str>| Stream {
            provider: "test".to_string(),
            title: "test".to_string(),
            quality: quality.map(String::from),
            size: None,
            size_bytes: 0,
            seeders: None,
            url: None,
            info_hash: None,
        };

        assert_eq!(make_stream(Some("2160p")).quality_rank(), 4);
        assert_eq!(make_stream(Some("4K")).quality_rank(), 4);
        assert_eq!(make_stream(Some("1080p")).quality_rank(), 3);
        assert_eq!(make_stream(Some("720p")).quality_rank(), 2);
        assert_eq!(make_stream(Some("480p")).quality_rank(), 1);
        assert_eq!(make_stream(None).quality_rank(), 0);
    }
}
