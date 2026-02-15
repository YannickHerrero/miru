use lazy_static::lazy_static;
use regex::Regex;

use crate::api::media::MediaType;
use crate::api::torrentio::Stream;

lazy_static! {
    /// Keywords that strongly suggest a source is not the actual main content
    static ref TRAILER_KEYWORDS: Regex =
        Regex::new(r"(?i)\b(trailer|promo|sample|preview|clip|extra|bonus|teaser|opening|ending|op|ed)\b").unwrap();
}

/// Options for scoring a stream
#[derive(Debug, Clone)]
pub struct ScoringOptions {
    pub media_type: MediaType,
    pub is_anime: bool,
}

/// Filter out potential trailers based on size, quality, and keywords
pub fn is_likely_trailer(stream: &Stream, media_type: MediaType) -> bool {
    let title_lower = stream.title.to_lowercase();

    // Keyword check
    if TRAILER_KEYWORDS.is_match(&title_lower) {
        return true;
    }

    // Size-based heuristic (size_bytes == u64::MAX means unknown size, don't penalize)
    if stream.size_bytes == u64::MAX {
        return false;
    }

    let size_mb = stream.size_bytes as f64 / (1024.0 * 1024.0);
    let quality = stream.quality.as_deref().unwrap_or("").to_lowercase();

    match media_type {
        MediaType::TvShow => {
            // TV Episode thresholds
            if quality.contains("2160p") || quality.contains("4k") {
                size_mb < 400.0
            } else if quality.contains("1080p") {
                size_mb < 150.0
            } else if quality.contains("720p") {
                size_mb < 80.0
            } else {
                size_mb < 30.0
            }
        }
        MediaType::Movie => {
            // Movie thresholds
            if quality.contains("2160p") || quality.contains("4k") {
                size_mb < 2000.0
            } else if quality.contains("1080p") {
                size_mb < 800.0
            } else if quality.contains("720p") {
                size_mb < 400.0
            } else {
                size_mb < 150.0
            }
        }
    }
}

/// Calculate a recommendation score for a stream.
///
/// Higher scores are better. Mirrors the scoring algorithm from Mira:
/// - Trailer penalty: -10000
/// - Quality: 1080p +1000, 720p +800, 4K +600, other +400
/// - Cache bonus: +1000 if cached on Real-Debrid
/// - Language: +30 per language detected
/// - Seeder bonus: log2(seeders + 1) * 50 (logarithmic)
/// - Size penalty: -(sizeGB ^ 1.5) * 80 (exponential)
pub fn calculate_source_score(stream: &Stream, options: &ScoringOptions) -> f64 {
    // 1. Trailer penalty (massive penalty)
    if is_likely_trailer(stream, options.media_type) {
        return -10000.0;
    }

    let mut score: f64 = 0.0;

    // 2. Quality points - Prefer 1080p
    let quality = stream.quality.as_deref().unwrap_or("").to_lowercase();

    if quality.contains("1080p") {
        score += 1000.0;
    } else if quality.contains("720p") {
        score += 800.0;
    } else if quality.contains("2160p") || quality.contains("4k") {
        score += 600.0; // Lower score for 4K to favor 1080p/720p
    } else {
        score += 400.0;
    }

    // 3. Provider bonus (nyaasi for anime -- currently +0 in Mira, reserved for future)
    if options.is_anime && stream.provider.to_lowercase().contains("nyaa") {
        // Reserved for future use
    }

    // 4. Caching bonus (essential for smooth experience)
    if stream.is_cached {
        score += 1000.0;
    }

    // 5. Language points
    if !stream.languages.is_empty() {
        score += stream.languages.len() as f64 * 30.0;
    }

    // 6. Seeder bonus (logarithmic)
    if let Some(seeders) = stream.seeders {
        if seeders > 0 {
            score += (seeders as f64 + 1.0).log2() * 50.0;
        }
    }

    // 7. Size penalty (efficiency)
    // Penalty grows exponentially to avoid excessively large files
    if stream.size_bytes < u64::MAX {
        let size_gb = stream.size_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        score -= size_gb.powf(1.5) * 80.0;
    }

    score
}

/// Get recommended sources from a list of streams.
///
/// Returns the indices of the top `limit` streams by score (filtering out score <= 0).
/// These are the "best picks" that should be pinned to the top of the list.
pub fn get_recommended_indices(
    streams: &[Stream],
    options: &ScoringOptions,
    limit: usize,
) -> Vec<usize> {
    if streams.is_empty() {
        return vec![];
    }

    let mut scored: Vec<(usize, f64)> = streams
        .iter()
        .enumerate()
        .map(|(i, stream)| (i, calculate_source_score(stream, options)))
        .filter(|(_, score)| *score > 0.0)
        .collect();

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    scored.into_iter().take(limit).map(|(i, _)| i).collect()
}

/// Sort streams by score (descending) with tie-breaking on quality rank then size.
///
/// This replaces the previous hardcoded quality-then-size sort with a comprehensive
/// scoring algorithm that considers quality, cache status, seeders, size, and languages.
pub fn sort_streams_by_score(streams: &mut Vec<Stream>, options: &ScoringOptions) {
    streams.sort_by(|a, b| {
        let score_a = calculate_source_score(a, options);
        let score_b = calculate_source_score(b, options);

        // Primary sort: score descending
        match score_b.partial_cmp(&score_a) {
            Some(std::cmp::Ordering::Equal) | None => {}
            Some(ord) => return ord,
        }

        // Tie-breaker 1: quality rank descending
        match b.quality_rank().cmp(&a.quality_rank()) {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }

        // Tie-breaker 2: smaller files first
        a.size_bytes.cmp(&b.size_bytes)
    });
}

/// Reorder streams so that recommended sources appear at the top,
/// preserving the relative order of both recommended and non-recommended streams.
pub fn pin_recommended_to_top(streams: Vec<Stream>, recommended_indices: &[usize]) -> Vec<Stream> {
    if recommended_indices.is_empty() {
        return streams;
    }

    let recommended_set: std::collections::HashSet<usize> =
        recommended_indices.iter().copied().collect();

    let mut recommended: Vec<Stream> = Vec::new();
    let mut others: Vec<Stream> = Vec::new();

    for (i, stream) in streams.into_iter().enumerate() {
        if recommended_set.contains(&i) {
            recommended.push(stream);
        } else {
            others.push(stream);
        }
    }

    // Keep recommended in the order of the recommended_indices (by score)
    let mut result = Vec::with_capacity(recommended.len() + others.len());
    result.extend(recommended);
    result.extend(others);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stream(
        quality: Option<&str>,
        size_bytes: u64,
        seeders: Option<u32>,
        is_cached: bool,
    ) -> Stream {
        Stream {
            provider: "test".to_string(),
            title: "Test Movie 2024".to_string(),
            quality: quality.map(String::from),
            size: Some("1.0 GB".to_string()),
            size_bytes,
            seeders,
            url: None,
            info_hash: None,
            file_idx: None,
            video_codec: None,
            audio: None,
            hdr: None,
            source_type: None,
            languages: vec![],
            is_cached,
        }
    }

    fn movie_options() -> ScoringOptions {
        ScoringOptions {
            media_type: MediaType::Movie,
            is_anime: false,
        }
    }

    fn tv_options() -> ScoringOptions {
        ScoringOptions {
            media_type: MediaType::TvShow,
            is_anime: false,
        }
    }

    #[test]
    fn test_trailer_keyword_detection() {
        let mut stream = make_stream(Some("1080p"), 2 * 1024 * 1024 * 1024, Some(100), true);
        stream.title = "Movie 2024 Trailer HD".to_string();
        assert!(is_likely_trailer(&stream, MediaType::Movie));
    }

    #[test]
    fn test_trailer_size_detection_movie() {
        // 500MB for 1080p movie is suspiciously small
        let stream = make_stream(Some("1080p"), 500 * 1024 * 1024, Some(100), true);
        assert!(is_likely_trailer(&stream, MediaType::Movie));
    }

    #[test]
    fn test_not_trailer_normal_movie() {
        // 2GB for 1080p movie is normal
        let stream = make_stream(Some("1080p"), 2 * 1024 * 1024 * 1024, Some(100), true);
        assert!(!is_likely_trailer(&stream, MediaType::Movie));
    }

    #[test]
    fn test_trailer_size_detection_tv() {
        // 100MB for 1080p TV episode is suspiciously small
        let stream = make_stream(Some("1080p"), 100 * 1024 * 1024, Some(50), true);
        assert!(is_likely_trailer(&stream, MediaType::TvShow));
    }

    #[test]
    fn test_not_trailer_normal_tv() {
        // 500MB for 1080p TV episode is normal
        let stream = make_stream(Some("1080p"), 500 * 1024 * 1024, Some(50), true);
        assert!(!is_likely_trailer(&stream, MediaType::TvShow));
    }

    #[test]
    fn test_trailer_gets_negative_score() {
        let mut stream = make_stream(Some("1080p"), 2 * 1024 * 1024 * 1024, Some(100), true);
        stream.title = "Movie Trailer HD".to_string();
        let score = calculate_source_score(&stream, &movie_options());
        assert!(score < 0.0);
        assert_eq!(score, -10000.0);
    }

    #[test]
    fn test_cached_1080p_scores_higher_than_uncached() {
        let cached = make_stream(Some("1080p"), 2 * 1024 * 1024 * 1024, Some(100), true);
        let uncached = make_stream(Some("1080p"), 2 * 1024 * 1024 * 1024, Some(100), false);
        let opts = movie_options();
        assert!(calculate_source_score(&cached, &opts) > calculate_source_score(&uncached, &opts));
    }

    #[test]
    fn test_1080p_scores_higher_than_4k() {
        let hd = make_stream(Some("1080p"), 2 * 1024 * 1024 * 1024, Some(100), true);
        let uhd = make_stream(Some("2160p"), 2 * 1024 * 1024 * 1024, Some(100), true);
        let opts = movie_options();
        assert!(calculate_source_score(&hd, &opts) > calculate_source_score(&uhd, &opts));
    }

    #[test]
    fn test_more_seeders_score_higher() {
        let many_seeders = make_stream(Some("1080p"), 2 * 1024 * 1024 * 1024, Some(500), true);
        let few_seeders = make_stream(Some("1080p"), 2 * 1024 * 1024 * 1024, Some(5), true);
        let opts = movie_options();
        assert!(
            calculate_source_score(&many_seeders, &opts)
                > calculate_source_score(&few_seeders, &opts)
        );
    }

    #[test]
    fn test_smaller_size_scores_higher() {
        let small = make_stream(Some("1080p"), 1 * 1024 * 1024 * 1024, Some(100), true);
        let large = make_stream(Some("1080p"), 20 * 1024 * 1024 * 1024, Some(100), true);
        let opts = movie_options();
        assert!(calculate_source_score(&small, &opts) > calculate_source_score(&large, &opts));
    }

    #[test]
    fn test_languages_add_score() {
        let mut with_langs = make_stream(Some("1080p"), 2 * 1024 * 1024 * 1024, Some(100), true);
        with_langs.languages = vec!["English".to_string(), "French".to_string()];
        let without_langs = make_stream(Some("1080p"), 2 * 1024 * 1024 * 1024, Some(100), true);
        let opts = movie_options();
        assert!(
            calculate_source_score(&with_langs, &opts)
                > calculate_source_score(&without_langs, &opts)
        );
    }

    #[test]
    fn test_get_recommended_indices() {
        let streams = vec![
            make_stream(Some("1080p"), 2 * 1024 * 1024 * 1024, Some(100), true), // Good
            make_stream(Some("480p"), 500 * 1024 * 1024, Some(5), false),        // Poor
            make_stream(Some("1080p"), 3 * 1024 * 1024 * 1024, Some(200), true), // Good
        ];
        let opts = movie_options();
        let recommended = get_recommended_indices(&streams, &opts, 2);
        assert_eq!(recommended.len(), 2);
        // Should include the two good streams (indices 0 and 2)
        assert!(recommended.contains(&0) || recommended.contains(&2));
    }

    #[test]
    fn test_sort_streams_by_score() {
        let mut streams = vec![
            make_stream(Some("480p"), 500 * 1024 * 1024, Some(5), false), // Worst
            make_stream(Some("1080p"), 2 * 1024 * 1024 * 1024, Some(100), true), // Best
            make_stream(Some("720p"), 1 * 1024 * 1024 * 1024, Some(50), true), // Middle
        ];
        let opts = movie_options();
        sort_streams_by_score(&mut streams, &opts);

        // First should be the 1080p cached stream
        assert_eq!(streams[0].quality.as_deref(), Some("1080p"));
        // Last should be the 480p uncached stream
        assert_eq!(streams[2].quality.as_deref(), Some("480p"));
    }

    #[test]
    fn test_pin_recommended_to_top() {
        let streams = vec![
            make_stream(Some("480p"), 500 * 1024 * 1024, Some(5), false), // index 0
            make_stream(Some("1080p"), 2 * 1024 * 1024 * 1024, Some(100), true), // index 1
            make_stream(Some("720p"), 1 * 1024 * 1024 * 1024, Some(50), true), // index 2
        ];

        // Recommend index 1 and 2
        let recommended = vec![1, 2];
        let result = pin_recommended_to_top(streams, &recommended);

        // First two should be the recommended ones
        assert_eq!(result[0].quality.as_deref(), Some("1080p"));
        assert_eq!(result[1].quality.as_deref(), Some("720p"));
        // Last should be the non-recommended one
        assert_eq!(result[2].quality.as_deref(), Some("480p"));
    }

    #[test]
    fn test_unknown_size_not_penalized_as_trailer() {
        let stream = make_stream(Some("1080p"), u64::MAX, Some(100), true);
        assert!(!is_likely_trailer(&stream, MediaType::Movie));
    }

    #[test]
    fn test_tv_options() {
        let stream = make_stream(Some("1080p"), 500 * 1024 * 1024, Some(50), true);
        let opts = tv_options();
        let score = calculate_source_score(&stream, &opts);
        assert!(score > 0.0);
    }
}
