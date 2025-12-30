use reqwest::Client;
use serde::Deserialize;

use crate::error::ApiError;

const ARM_SERVER_URL: &str = "https://arm.haglund.dev/api/v2";

/// ID mapping client using arm-server
pub struct MappingClient {
    client: Client,
}

impl MappingClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Convert MAL ID to IMDB ID
    pub async fn mal_to_imdb(&self, mal_id: i32) -> Result<String, ApiError> {
        let url = format!("{}/ids?source=myanimelist&id={}", ARM_SERVER_URL, mal_id);

        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ApiError::MappingNotFound);
        }

        if !response.status().is_success() {
            return Err(ApiError::Mapping(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let data: MappingResponse = response.json().await.map_err(|e| {
            ApiError::Mapping(format!("Failed to parse response: {}", e))
        })?;

        data.imdb.ok_or(ApiError::MappingNotFound)
    }

    /// Convert Anilist ID to IMDB ID (via MAL ID)
    pub async fn anilist_to_imdb(&self, anilist_id: i32, mal_id: Option<i32>) -> Result<String, ApiError> {
        // If we have MAL ID, use it directly
        if let Some(mal) = mal_id {
            return self.mal_to_imdb(mal).await;
        }

        // Try to get MAL ID from Anilist ID
        let url = format!("{}/ids?source=anilist&id={}", ARM_SERVER_URL, anilist_id);

        let response = self.client.get(&url).send().await?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(ApiError::MappingNotFound);
        }

        if !response.status().is_success() {
            return Err(ApiError::Mapping(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let data: MappingResponse = response.json().await.map_err(|e| {
            ApiError::Mapping(format!("Failed to parse response: {}", e))
        })?;

        data.imdb.ok_or(ApiError::MappingNotFound)
    }
}

impl Default for MappingClient {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct MappingResponse {
    imdb: Option<String>,
}
