use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::api::media::{Media, MediaSource, MediaType};
use crate::error::ApiError;

const ANILIST_URL: &str = "https://graphql.anilist.co";

/// Anilist GraphQL client
pub struct AnilistClient {
    client: Client,
}

impl AnilistClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Search for anime by title
    pub async fn search_anime(&self, query: &str) -> Result<Vec<Anime>, ApiError> {
        let graphql_query = r#"
            query ($search: String) {
                Page(perPage: 10) {
                    media(search: $search, type: ANIME, sort: POPULARITY_DESC) {
                        id
                        idMal
                        title {
                            romaji
                            english
                        }
                        description(asHtml: false)
                        episodes
                        averageScore
                        seasonYear
                        status
                        format
                        genres
                        coverImage {
                            medium
                        }
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "search": query
        });

        let response = self
            .client
            .post(ANILIST_URL)
            .json(&serde_json::json!({
                "query": graphql_query,
                "variables": variables
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ApiError::Anilist(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let data: AnilistResponse = response.json().await?;

        if let Some(errors) = data.errors {
            if let Some(first_error) = errors.first() {
                return Err(ApiError::Anilist(first_error.message.clone()));
            }
        }

        let media = data
            .data
            .ok_or_else(|| ApiError::Anilist("No data in response".to_string()))?
            .page
            .media;

        Ok(media.into_iter().map(Anime::from).collect())
    }
}

impl Default for AnilistClient {
    fn default() -> Self {
        Self::new()
    }
}

// Response types for deserialization
#[derive(Debug, Deserialize)]
struct AnilistResponse {
    data: Option<AnilistData>,
    errors: Option<Vec<AnilistError>>,
}

#[derive(Debug, Deserialize)]
struct AnilistData {
    #[serde(rename = "Page")]
    page: AnilistPage,
}

#[derive(Debug, Deserialize)]
struct AnilistPage {
    media: Vec<MediaResponse>,
}

#[derive(Debug, Deserialize)]
struct AnilistError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct MediaResponse {
    id: i32,
    #[serde(rename = "idMal")]
    id_mal: Option<i32>,
    title: TitleResponse,
    description: Option<String>,
    episodes: Option<i32>,
    #[serde(rename = "averageScore")]
    average_score: Option<i32>,
    #[serde(rename = "seasonYear")]
    season_year: Option<i32>,
    status: Option<String>,
    format: Option<String>,
    #[serde(default)]
    genres: Vec<String>,
    #[serde(rename = "coverImage")]
    cover_image: Option<CoverImageResponse>,
}

#[derive(Debug, Deserialize)]
struct TitleResponse {
    romaji: Option<String>,
    english: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CoverImageResponse {
    medium: Option<String>,
}

/// Anime data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anime {
    pub id: i32,
    pub id_mal: Option<i32>,
    pub title: String,
    pub title_english: Option<String>,
    pub description: Option<String>,
    pub episodes: Option<i32>,
    pub score: Option<f32>,
    pub year: Option<i32>,
    pub status: Option<String>,
    pub format: Option<String>,
    pub genres: Vec<String>,
    pub cover_image: Option<String>,
}

impl From<MediaResponse> for Anime {
    fn from(media: MediaResponse) -> Self {
        let title = media
            .title
            .english
            .clone()
            .or(media.title.romaji.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Clean up description - strip any remaining HTML-like content and normalize whitespace
        let description = media.description.map(|d| {
            // Remove any HTML tags that might slip through
            let re = regex::Regex::new(r"<[^>]+>").unwrap();
            let cleaned = re.replace_all(&d, "");
            // Normalize whitespace and newlines
            cleaned
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        });

        Self {
            id: media.id,
            id_mal: media.id_mal,
            title,
            title_english: media.title.english,
            description,
            episodes: media.episodes,
            score: media.average_score.map(|s| s as f32 / 10.0),
            year: media.season_year,
            status: media.status,
            format: media.format,
            genres: media.genres,
            cover_image: media.cover_image.and_then(|c| c.medium),
        }
    }
}

/// Convert Anime to the unified Media type
impl From<Anime> for Media {
    fn from(anime: Anime) -> Self {
        Self {
            media_type: MediaType::Anime,
            source: MediaSource::AniList {
                id: anime.id,
                id_mal: anime.id_mal,
            },
            title: anime.title_english.clone().unwrap_or_else(|| anime.title.clone()),
            title_original: Some(anime.title),
            imdb_id: None, // Resolved via ARM server when needed
            year: anime.year,
            score: anime.score,
            episodes: anime.episodes,
            seasons: None, // Anime typically doesn't use seasons in this context
            cover_image: anime.cover_image,
            episode_titles: vec![],
            description: anime.description,
            status: anime.status,
            format: anime.format,
            genres: anime.genres,
        }
    }
}
