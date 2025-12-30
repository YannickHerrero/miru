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
    // Match HDR patterns
    static ref HDR_RE: Regex = Regex::new(r"(?i)\b(HDR10\+|HDR10|DoVi|DV|Dolby[\s.]?Vision|HDR)\b").unwrap();
    // Match video codec patterns
    static ref VIDEO_CODEC_RE: Regex = Regex::new(r"(?i)\b(HEVC|x265|x264|AVC|AV1|H\.?265|H\.?264|VC-1|10bit|10-bit)\b").unwrap();
    // Match audio codec patterns (simplified)
    static ref AUDIO_RE: Regex = Regex::new(r"(?i)(DTS-HD[\s.]?MA|TrueHD|Atmos|DTS|AAC|FLAC|EAC3|E-AC-3|AC3|DD|DD\+|LPCM)[\s.]?(\d\.\d)?").unwrap();
    // Match audio channels
    static ref AUDIO_CHANNELS_RE: Regex = Regex::new(r"\b([257]\.[01])\b").unwrap();
    // Match source type patterns
    static ref SOURCE_RE: Regex = Regex::new(r"(?i)\b(UHD[\s.]?BluRay|BluRay|Blu-Ray|BDRip|BRRip|WEB-DL|WEBDL|WEBRip|REMUX|HDTV|DVDRip)\b").unwrap();
    // Match language flags
    static ref LANG_FLAGS_RE: Regex = Regex::new(r"(🇬🇧|🇺🇸|🇩🇪|🇫🇷|🇮🇹|🇪🇸|🇯🇵|🇰🇷|🇨🇳|🇧🇷|🇵🇹|🇷🇺|🇳🇱|🇵🇱|🇸🇪|🇳🇴|🇩🇰|🇫🇮|🇬🇷|🇹🇷|🇮🇳|🇹🇭|🇻🇳|🇮🇩|🇲🇽|🇦🇷)").unwrap();
}

/// Convert flag emoji to language name
fn flag_to_language(flag: &str) -> &'static str {
    match flag {
        "🇬🇧" | "🇺🇸" => "English",
        "🇩🇪" => "German",
        "🇫🇷" => "French",
        "🇮🇹" => "Italian",
        "🇪🇸" | "🇲🇽" | "🇦🇷" => "Spanish",
        "🇯🇵" => "Japanese",
        "🇰🇷" => "Korean",
        "🇨🇳" => "Chinese",
        "🇧🇷" | "🇵🇹" => "Portuguese",
        "🇷🇺" => "Russian",
        "🇳🇱" => "Dutch",
        "🇵🇱" => "Polish",
        "🇸🇪" => "Swedish",
        "🇳🇴" => "Norwegian",
        "🇩🇰" => "Danish",
        "🇫🇮" => "Finnish",
        "🇬🇷" => "Greek",
        "🇹🇷" => "Turkish",
        "🇮🇳" => "Hindi",
        "🇹🇭" => "Thai",
        "🇻🇳" => "Vietnamese",
        "🇮🇩" => "Indonesian",
        _ => "Unknown",
    }
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
    fn build_config_string(&self, show_uncached: bool) -> String {
        let providers = self.config.providers.join(",");
        // debridoptions=nodownloadlinks ensures only cached/instant streams are returned
        // This means all URLs are direct RD links that can be played immediately
        // When show_uncached is true, we omit this option to show all available torrents
        if show_uncached {
            format!(
                "providers={}|sort=qualitysize|qualityfilter=scr,cam|realdebrid={}",
                providers, self.rd_api_key
            )
        } else {
            format!(
                "providers={}|sort=qualitysize|qualityfilter=scr,cam|debridoptions=nodownloadlinks|realdebrid={}",
                providers, self.rd_api_key
            )
        }
    }

    /// Get streams for a series episode
    /// When `show_uncached` is true, returns all available torrents including uncached ones
    pub async fn get_streams(
        &self,
        imdb_id: &str,
        season: u32,
        episode: u32,
        show_uncached: bool,
    ) -> Result<Vec<Stream>, ApiError> {
        let config_str = self.build_config_string(show_uncached);
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
    /// When `show_uncached` is true, returns all available torrents including uncached ones
    pub async fn get_movie_streams(&self, imdb_id: &str, show_uncached: bool) -> Result<Vec<Stream>, ApiError> {
        let config_str = self.build_config_string(show_uncached);
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
}

/// Parsed stream data
#[derive(Debug, Clone)]
pub struct Stream {
    /// Provider name (e.g., "nyaasi", "1337x")
    pub provider: String,
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
    /// Video codec (e.g., "HEVC", "x264", "AV1")
    pub video_codec: Option<String>,
    /// Audio format (e.g., "DTS-HD MA 7.1", "TrueHD Atmos")
    pub audio: Option<String>,
    /// HDR type (e.g., "HDR", "DV", "HDR10+")
    pub hdr: Option<String>,
    /// Source type (e.g., "BluRay", "WEB-DL", "REMUX")
    pub source_type: Option<String>,
    /// Available languages (flag emojis)
    pub languages: Vec<String>,
    /// Whether this stream is cached on Real-Debrid (instant playback)
    pub is_cached: bool,
}

impl Stream {
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
        // Combine name and title for parsing (name often has quality info like "4k DV | HDR")
        let combined = format!("{}\n{}", resp.name, resp.title);

        // Detect if stream is cached based on name prefix
        // [RD+] = cached, [RD download] or [RD] without + = uncached
        let is_cached = resp.name.contains("[RD+]") || resp.name.contains("[⚡]");

        // Parse provider from name (e.g., "[RD+] nyaasi" -> "nyaasi")
        let provider = resp
            .name
            .split(']')
            .next_back()
            .map(|s| s.trim())
            .unwrap_or(&resp.name)
            .to_string();

        // Parse quality from title
        let quality = QUALITY_RE
            .find(&combined)
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

        // Parse HDR type
        let hdr = parse_hdr(&combined);

        // Parse video codec
        let video_codec = parse_video_codec(&combined);

        // Parse audio format
        let audio = parse_audio(&combined);

        // Parse source type
        let source_type = SOURCE_RE
            .find(&combined)
            .map(|m| normalize_source(m.as_str()));

        // Parse language flags and convert to language names
        let languages: Vec<String> = LANG_FLAGS_RE
            .find_iter(&resp.title)
            .map(|m| flag_to_language(m.as_str()).to_string())
            .collect::<std::collections::HashSet<_>>() // Deduplicate (e.g., 🇬🇧 and 🇺🇸 both map to English)
            .into_iter()
            .collect();

        Self {
            provider,
            quality,
            size,
            size_bytes,
            seeders,
            url: resp.url,
            video_codec,
            audio,
            hdr,
            source_type,
            languages,
            is_cached,
        }
    }
}

/// Parse HDR type from title, normalizing variants
fn parse_hdr(text: &str) -> Option<String> {
    let mut hdr_types = Vec::new();
    
    for cap in HDR_RE.find_iter(text) {
        let hdr = match cap.as_str().to_uppercase().as_str() {
            "DOVI" | "DV" | "DOLBYVISION" | "DOLBY VISION" | "DOLBY.VISION" => "DV",
            "HDR10+" => "HDR10+",
            "HDR10" => "HDR10",
            "HDR" => "HDR",
            _ => continue,
        };
        if !hdr_types.contains(&hdr.to_string()) {
            hdr_types.push(hdr.to_string());
        }
    }
    
    if hdr_types.is_empty() {
        None
    } else {
        Some(hdr_types.join(" / "))
    }
}

/// Parse video codec from title, normalizing variants
fn parse_video_codec(text: &str) -> Option<String> {
    let mut codecs = Vec::new();
    
    for cap in VIDEO_CODEC_RE.find_iter(text) {
        let codec = match cap.as_str().to_uppercase().replace('.', "").as_str() {
            "HEVC" | "H265" | "X265" => "HEVC",
            "AVC" | "H264" | "X264" => "AVC",
            "AV1" => "AV1",
            "VC-1" => "VC-1",
            "10BIT" | "10-BIT" => "10bit",
            _ => continue,
        };
        if !codecs.contains(&codec.to_string()) {
            codecs.push(codec.to_string());
        }
    }
    
    if codecs.is_empty() {
        None
    } else {
        Some(codecs.join(" "))
    }
}

/// Parse audio format from title
fn parse_audio(text: &str) -> Option<String> {
    let mut audio_parts = Vec::new();
    
    // Find audio codec
    if let Some(cap) = AUDIO_RE.find(text) {
        let codec = cap.as_str().to_uppercase();
        let normalized = if codec.contains("DTS-HD") || codec.contains("DTS HD") {
            "DTS-HD MA"
        } else if codec.contains("TRUEHD") {
            "TrueHD"
        } else if codec.contains("ATMOS") {
            "Atmos"
        } else if codec.contains("EAC3") || codec.contains("E-AC-3") || codec.contains("DD+") {
            "EAC3"
        } else if codec.contains("AC3") || codec.contains("DD") {
            "AC3"
        } else if codec.contains("AAC") {
            "AAC"
        } else if codec.contains("FLAC") {
            "FLAC"
        } else if codec.contains("DTS") {
            "DTS"
        } else if codec.contains("LPCM") {
            "LPCM"
        } else {
            return None;
        };
        audio_parts.push(normalized.to_string());
    }
    
    // Check for Atmos separately (can appear with other codecs like TrueHD Atmos)
    if text.to_uppercase().contains("ATMOS") && !audio_parts.contains(&"Atmos".to_string()) {
        audio_parts.push("Atmos".to_string());
    }
    
    // Find channel configuration
    if let Some(cap) = AUDIO_CHANNELS_RE.find(text) {
        audio_parts.push(cap.as_str().to_string());
    }
    
    if audio_parts.is_empty() {
        None
    } else {
        Some(audio_parts.join(" "))
    }
}

/// Normalize source type string
fn normalize_source(source: &str) -> String {
    match source.to_uppercase().replace(['-', ' ', '.'], "").as_str() {
        "UHDBLURAY" => "UHD BluRay".to_string(),
        "BLURAY" => "BluRay".to_string(),
        "BDRIP" | "BRRIP" => "BDRip".to_string(),
        "WEBDL" => "WEB-DL".to_string(),
        "WEBRIP" => "WEBRip".to_string(),
        "REMUX" => "REMUX".to_string(),
        "HDTV" => "HDTV".to_string(),
        "DVDRIP" => "DVDRip".to_string(),
        _ => source.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_stream(quality: Option<&str>) -> Stream {
        Stream {
            provider: "test".to_string(),
            quality: quality.map(String::from),
            size: None,
            size_bytes: 0,
            seeders: None,
            url: None,
            video_codec: None,
            audio: None,
            hdr: None,
            source_type: None,
            languages: vec![],
            is_cached: true,
        }
    }

    #[test]
    fn test_parse_stream_title() {
        let resp = StreamResponse {
            name: "[RD+] nyaasi".to_string(),
            title: "Frieren S01E01 1080p WEB x264\n👤 150 💾 1.2 GB".to_string(),
            url: Some("https://example.com".to_string()),
        };

        let stream = Stream::from(resp);

        assert_eq!(stream.provider, "nyaasi");
        assert_eq!(stream.quality, Some("1080p".to_string()));
        assert_eq!(stream.size, Some("1.2 GB".to_string()));
        assert_eq!(stream.seeders, Some(150));
        assert_eq!(stream.quality_rank(), 3);
        assert_eq!(stream.video_codec, Some("AVC".to_string())); // x264 -> AVC
    }

    #[test]
    fn test_parse_stream_720p() {
        let resp = StreamResponse {
            name: "[RD+] 1337x".to_string(),
            title: "Some Anime 720p\n👤 50 💾 800 MB".to_string(),
            url: None,
        };

        let stream = Stream::from(resp);

        assert_eq!(stream.provider, "1337x");
        assert_eq!(stream.quality, Some("720p".to_string()));
        assert_eq!(stream.size, Some("800 MB".to_string()));
        assert_eq!(stream.seeders, Some(50));
        assert_eq!(stream.quality_rank(), 2);
    }

    #[test]
    fn test_parse_stream_with_hdr_and_audio() {
        let resp = StreamResponse {
            name: "Torrentio\n4k DV | HDR".to_string(),
            title: "Movie.2024.2160p.UHD.BluRay.REMUX.HEVC.DTS-HD.MA.7.1-GROUP\n👤 25 💾 45.5 GB".to_string(),
            url: Some("https://example.com".to_string()),
        };

        let stream = Stream::from(resp);

        assert_eq!(stream.quality, Some("2160p".to_string()));
        assert_eq!(stream.hdr, Some("DV / HDR".to_string()));
        assert_eq!(stream.video_codec, Some("HEVC".to_string()));
        assert_eq!(stream.audio, Some("DTS-HD MA 7.1".to_string()));
        // UHD BluRay matches first in the regex
        assert_eq!(stream.source_type, Some("UHD BluRay".to_string()));
    }

    #[test]
    fn test_parse_stream_with_languages() {
        let resp = StreamResponse {
            name: "Torrentio\n1080p".to_string(),
            title: "Movie.2024.1080p.BluRay.x265\n👤 10 💾 2.5 GB\n🇬🇧 / 🇩🇪".to_string(),
            url: None,
        };

        let stream = Stream::from(resp);

        // Languages are deduplicated and converted to names
        assert!(stream.languages.contains(&"English".to_string()));
        assert!(stream.languages.contains(&"German".to_string()));
        assert_eq!(stream.languages.len(), 2);
        assert_eq!(stream.source_type, Some("BluRay".to_string()));
        assert_eq!(stream.video_codec, Some("HEVC".to_string())); // x265 -> HEVC
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
        assert_eq!(make_test_stream(Some("2160p")).quality_rank(), 4);
        assert_eq!(make_test_stream(Some("4K")).quality_rank(), 4);
        assert_eq!(make_test_stream(Some("1080p")).quality_rank(), 3);
        assert_eq!(make_test_stream(Some("720p")).quality_rank(), 2);
        assert_eq!(make_test_stream(Some("480p")).quality_rank(), 1);
        assert_eq!(make_test_stream(None).quality_rank(), 0);
    }

    #[test]
    fn test_cached_detection() {
        // Cached stream with [RD+]
        let resp = StreamResponse {
            name: "[RD+] nyaasi".to_string(),
            title: "Anime 1080p".to_string(),
            url: Some("https://example.com".to_string()),
        };
        assert!(Stream::from(resp).is_cached);

        // Uncached stream without [RD+]
        let resp = StreamResponse {
            name: "[RD download] nyaasi".to_string(),
            title: "Anime 1080p".to_string(),
            url: None,
        };
        assert!(!Stream::from(resp).is_cached);

        // Cached stream with lightning bolt
        let resp = StreamResponse {
            name: "[⚡] 1337x".to_string(),
            title: "Movie 1080p".to_string(),
            url: Some("https://example.com".to_string()),
        };
        assert!(Stream::from(resp).is_cached);
    }
}
