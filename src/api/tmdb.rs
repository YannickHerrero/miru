use reqwest::Client;
use serde::Deserialize;

use crate::api::media::{Media, MediaSource, MediaType, Season};
use crate::error::ApiError;

const TMDB_API_URL: &str = "https://api.themoviedb.org/3";
const TMDB_IMAGE_BASE: &str = "https://image.tmdb.org/t/p/w185";

/// Map TMDB genre IDs to genre names
fn genre_name(id: i32) -> Option<&'static str> {
    match id {
        28 => Some("Action"),
        12 => Some("Adventure"),
        16 => Some("Animation"),
        35 => Some("Comedy"),
        80 => Some("Crime"),
        99 => Some("Documentary"),
        18 => Some("Drama"),
        10751 => Some("Family"),
        14 => Some("Fantasy"),
        36 => Some("History"),
        27 => Some("Horror"),
        10402 => Some("Music"),
        9648 => Some("Mystery"),
        10749 => Some("Romance"),
        878 => Some("Sci-Fi"),
        10770 => Some("TV Movie"),
        53 => Some("Thriller"),
        10752 => Some("War"),
        37 => Some("Western"),
        // TV-specific genres
        10759 => Some("Action & Adventure"),
        10762 => Some("Kids"),
        10763 => Some("News"),
        10764 => Some("Reality"),
        10765 => Some("Sci-Fi & Fantasy"),
        10766 => Some("Soap"),
        10767 => Some("Talk"),
        10768 => Some("War & Politics"),
        _ => None,
    }
}

/// Convert genre IDs to genre names
fn genres_from_ids(ids: &[i32]) -> Vec<String> {
    ids.iter()
        .filter_map(|&id| genre_name(id).map(String::from))
        .collect()
}

/// TMDB API client
pub struct TmdbClient {
    client: Client,
    api_key: String,
}

impl TmdbClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    /// Check if the client is configured (has API key)
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    /// Search for movies
    pub async fn search_movies(&self, query: &str) -> Result<Vec<Media>, ApiError> {
        if !self.is_configured() {
            return Ok(vec![]);
        }

        let url = format!(
            "{}/search/movie?api_key={}&query={}&include_adult=false",
            TMDB_API_URL,
            self.api_key,
            urlencoding::encode(query)
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ApiError::Tmdb(format!("HTTP {}", response.status())));
        }

        let data: MovieSearchResponse = response.json().await.map_err(|e| {
            ApiError::Tmdb(format!("Failed to parse response: {}", e))
        })?;

        Ok(data.results.into_iter().map(Media::from).collect())
    }

    /// Search for TV shows (excluding animation genre to avoid anime duplicates)
    pub async fn search_tv(&self, query: &str) -> Result<Vec<Media>, ApiError> {
        if !self.is_configured() {
            return Ok(vec![]);
        }

        let url = format!(
            "{}/search/tv?api_key={}&query={}&include_adult=false",
            TMDB_API_URL,
            self.api_key,
            urlencoding::encode(query)
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ApiError::Tmdb(format!("HTTP {}", response.status())));
        }

        let data: TvSearchResponse = response.json().await.map_err(|e| {
            ApiError::Tmdb(format!("Failed to parse response: {}", e))
        })?;

        // Filter out animation (genre_id 16) to avoid anime duplicates
        // Animation genre is typically anime on TMDB when origin_country includes JP
        let filtered: Vec<_> = data
            .results
            .into_iter()
            .filter(|tv| {
                let is_animation = tv.genre_ids.contains(&16);
                let is_japanese = tv.origin_country.iter().any(|c| c == "JP");
                // Exclude if it's Japanese animation (likely anime)
                !(is_animation && is_japanese)
            })
            .collect();

        Ok(filtered.into_iter().map(Media::from).collect())
    }

    /// Search for both movies and TV shows
    pub async fn search_all(&self, query: &str) -> Result<Vec<Media>, ApiError> {
        if !self.is_configured() {
            return Ok(vec![]);
        }

        // Search movies and TV in parallel
        let (movies, tv_shows) = tokio::join!(
            self.search_movies(query),
            self.search_tv(query)
        );

        let mut results = movies?;
        results.extend(tv_shows?);

        // Sort by popularity (approximated by vote_count, which is preserved in score)
        // Movies and TV shows are already sorted by popularity from TMDB
        Ok(results)
    }

    /// Get external IDs for a movie (to get IMDB ID)
    pub async fn get_movie_external_ids(&self, movie_id: i32) -> Result<String, ApiError> {
        let url = format!(
            "{}/movie/{}/external_ids?api_key={}",
            TMDB_API_URL, movie_id, self.api_key
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ApiError::Tmdb(format!("HTTP {}", response.status())));
        }

        let data: ExternalIdsResponse = response.json().await.map_err(|e| {
            ApiError::Tmdb(format!("Failed to parse response: {}", e))
        })?;

        data.imdb_id.ok_or(ApiError::MappingNotFound)
    }

    /// Get external IDs for a TV show (to get IMDB ID)
    pub async fn get_tv_external_ids(&self, tv_id: i32) -> Result<String, ApiError> {
        let url = format!(
            "{}/tv/{}/external_ids?api_key={}",
            TMDB_API_URL, tv_id, self.api_key
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ApiError::Tmdb(format!("HTTP {}", response.status())));
        }

        let data: ExternalIdsResponse = response.json().await.map_err(|e| {
            ApiError::Tmdb(format!("Failed to parse response: {}", e))
        })?;

        data.imdb_id.ok_or(ApiError::MappingNotFound)
    }

    /// Get TV show details including seasons
    pub async fn get_tv_details(&self, tv_id: i32) -> Result<Vec<Season>, ApiError> {
        let url = format!(
            "{}/tv/{}?api_key={}",
            TMDB_API_URL, tv_id, self.api_key
        );

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ApiError::Tmdb(format!("HTTP {}", response.status())));
        }

        let data: TvDetailsResponse = response.json().await.map_err(|e| {
            ApiError::Tmdb(format!("Failed to parse response: {}", e))
        })?;

        Ok(data
            .seasons
            .into_iter()
            .filter(|s| s.season_number > 0) // Exclude specials (season 0)
            .map(|s| Season {
                number: s.season_number,
                name: s.name,
                episode_count: s.episode_count,
            })
            .collect())
    }
}

impl Default for TmdbClient {
    fn default() -> Self {
        Self::new(String::new())
    }
}

// Response types for TMDB API

#[derive(Debug, Deserialize)]
struct MovieSearchResponse {
    results: Vec<MovieResult>,
}

#[derive(Debug, Deserialize)]
struct MovieResult {
    id: i32,
    title: String,
    original_title: Option<String>,
    release_date: Option<String>,
    vote_average: Option<f32>,
    poster_path: Option<String>,
    overview: Option<String>,
    #[serde(default)]
    genre_ids: Vec<i32>,
}

#[derive(Debug, Deserialize)]
struct TvSearchResponse {
    results: Vec<TvResult>,
}

#[derive(Debug, Deserialize)]
struct TvResult {
    id: i32,
    name: String,
    original_name: Option<String>,
    first_air_date: Option<String>,
    vote_average: Option<f32>,
    poster_path: Option<String>,
    overview: Option<String>,
    #[serde(default)]
    genre_ids: Vec<i32>,
    #[serde(default)]
    origin_country: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExternalIdsResponse {
    imdb_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TvDetailsResponse {
    #[serde(default)]
    seasons: Vec<SeasonInfo>,
    #[allow(dead_code)]
    number_of_seasons: Option<i32>,
    #[allow(dead_code)]
    number_of_episodes: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct SeasonInfo {
    season_number: u32,
    name: String,
    episode_count: u32,
}

// Conversion implementations

impl From<MovieResult> for Media {
    fn from(movie: MovieResult) -> Self {
        let year = movie
            .release_date
            .as_ref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());

        Self {
            media_type: MediaType::Movie,
            source: MediaSource::Tmdb { id: movie.id },
            title: movie.title,
            title_original: movie.original_title,
            imdb_id: None, // Fetched separately when needed
            year,
            score: movie.vote_average,
            episodes: None,
            seasons: None,
            cover_image: movie.poster_path.map(|p| format!("{}{}", TMDB_IMAGE_BASE, p)),
            episode_titles: vec![],
            description: movie.overview,
            status: Some("Released".to_string()),
            format: Some("Movie".to_string()),
            genres: genres_from_ids(&movie.genre_ids),
        }
    }
}

impl From<TvResult> for Media {
    fn from(tv: TvResult) -> Self {
        let year = tv
            .first_air_date
            .as_ref()
            .and_then(|d| d.split('-').next())
            .and_then(|y| y.parse().ok());

        Self {
            media_type: MediaType::TvShow,
            source: MediaSource::Tmdb { id: tv.id },
            title: tv.name,
            title_original: tv.original_name,
            imdb_id: None, // Fetched separately when needed
            year,
            score: tv.vote_average,
            episodes: None, // Fetched with details
            seasons: None,  // Fetched with details
            cover_image: tv.poster_path.map(|p| format!("{}{}", TMDB_IMAGE_BASE, p)),
            episode_titles: vec![],
            description: tv.overview,
            status: None, // Would need additional API call
            format: Some("TV".to_string()),
            genres: genres_from_ids(&tv.genre_ids),
        }
    }
}
