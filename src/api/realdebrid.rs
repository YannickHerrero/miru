use reqwest::Client;
use serde::Deserialize;

use crate::error::ApiError;

const RD_API_URL: &str = "https://api.real-debrid.com/rest/1.0";

/// Real-Debrid API client
pub struct RealDebridClient {
    client: Client,
    api_key: String,
}

impl RealDebridClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    /// Make an authenticated request
    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    /// Validate the API key by fetching user info
    pub async fn validate_key(&self) -> Result<RealDebridUser, ApiError> {
        let url = format!("{}/user", RD_API_URL);

        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ApiError::RealDebridAuth);
        }

        if !response.status().is_success() {
            return Err(ApiError::RealDebrid(format!(
                "HTTP {}",
                response.status()
            )));
        }

        let user: RealDebridUser = response.json().await?;
        Ok(user)
    }
}

#[derive(Debug, Deserialize)]
pub struct RealDebridUser {
    pub username: String,
}
