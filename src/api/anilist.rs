use reqwest::Client;
use serde::{Deserialize, Serialize};

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
                            native
                        }
                        episodes
                        averageScore
                        seasonYear
                        status
                        format
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

    /// Get anime details by ID
    #[allow(dead_code)]
    pub async fn get_anime(&self, id: i32) -> Result<Anime, ApiError> {
        let graphql_query = r#"
            query ($id: Int) {
                Media(id: $id, type: ANIME) {
                    id
                    idMal
                    title {
                        romaji
                        english
                        native
                    }
                    episodes
                    averageScore
                    seasonYear
                    status
                    format
                    coverImage {
                        medium
                    }
                    streamingEpisodes {
                        title
                        thumbnail
                    }
                }
            }
        "#;

        let variables = serde_json::json!({
            "id": id
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

        let data: AnilistSingleResponse = response.json().await?;

        if let Some(errors) = data.errors {
            if let Some(first_error) = errors.first() {
                return Err(ApiError::Anilist(first_error.message.clone()));
            }
        }

        let media = data
            .data
            .ok_or_else(|| ApiError::Anilist("No data in response".to_string()))?
            .media;

        Ok(Anime::from(media))
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
struct AnilistSingleResponse {
    data: Option<AnilistSingleData>,
    errors: Option<Vec<AnilistError>>,
}

#[derive(Debug, Deserialize)]
struct AnilistData {
    #[serde(rename = "Page")]
    page: AnilistPage,
}

#[derive(Debug, Deserialize)]
struct AnilistSingleData {
    #[serde(rename = "Media")]
    media: MediaResponse,
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
    episodes: Option<i32>,
    #[serde(rename = "averageScore")]
    average_score: Option<i32>,
    #[serde(rename = "seasonYear")]
    season_year: Option<i32>,
    status: Option<String>,
    format: Option<String>,
    #[serde(rename = "coverImage")]
    cover_image: Option<CoverImageResponse>,
    #[serde(rename = "streamingEpisodes")]
    streaming_episodes: Option<Vec<StreamingEpisodeResponse>>,
}

#[derive(Debug, Deserialize)]
struct TitleResponse {
    romaji: Option<String>,
    english: Option<String>,
    #[allow(dead_code)]
    native: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CoverImageResponse {
    medium: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamingEpisodeResponse {
    title: Option<String>,
    #[allow(dead_code)]
    thumbnail: Option<String>,
}

/// Anime data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anime {
    pub id: i32,
    pub id_mal: Option<i32>,
    pub title: String,
    pub title_english: Option<String>,
    #[allow(dead_code)]
    pub title_native: Option<String>,
    pub episodes: Option<i32>,
    pub score: Option<f32>,
    pub year: Option<i32>,
    #[allow(dead_code)]
    pub status: Option<String>,
    #[allow(dead_code)]
    pub format: Option<String>,
    #[allow(dead_code)]
    pub cover_image: Option<String>,
    pub episode_titles: Vec<String>,
}

impl Anime {
    /// Get the best display title
    pub fn display_title(&self) -> &str {
        self.title_english.as_deref().unwrap_or(&self.title)
    }

    /// Get episode list (either from streaming episodes or generated)
    pub fn get_episodes(&self) -> Vec<Episode> {
        let count = self.episodes.unwrap_or(0) as usize;

        if !self.episode_titles.is_empty() {
            self.episode_titles
                .iter()
                .enumerate()
                .map(|(i, title)| Episode {
                    number: i as u32 + 1,
                    title: title.clone(),
                })
                .collect()
        } else {
            (1..=count)
                .map(|n| Episode {
                    number: n as u32,
                    title: format!("Episode {}", n),
                })
                .collect()
        }
    }
}

impl From<MediaResponse> for Anime {
    fn from(media: MediaResponse) -> Self {
        let title = media
            .title
            .english
            .clone()
            .or(media.title.romaji.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let episode_titles = media
            .streaming_episodes
            .unwrap_or_default()
            .into_iter()
            .filter_map(|ep| ep.title)
            .collect();

        Self {
            id: media.id,
            id_mal: media.id_mal,
            title,
            title_english: media.title.english,
            title_native: media.title.native,
            episodes: media.episodes,
            score: media.average_score.map(|s| s as f32 / 10.0),
            year: media.season_year,
            status: media.status,
            format: media.format,
            cover_image: media.cover_image.and_then(|c| c.medium),
            episode_titles,
        }
    }
}

/// Episode data structure
#[derive(Debug, Clone)]
pub struct Episode {
    pub number: u32,
    pub title: String,
}
