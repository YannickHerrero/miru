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

    /// Unrestrict a link to get direct download URL
    #[allow(dead_code)]
    pub async fn unrestrict_link(&self, link: &str) -> Result<UnrestrictedLink, ApiError> {
        let url = format!("{}/unrestrict/link", RD_API_URL);

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .form(&[("link", link)])
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ApiError::RealDebridAuth);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ApiError::RealDebrid(format!(
                "Failed to unrestrict link: {}",
                error_text
            )));
        }

        let result: UnrestrictedLink = response.json().await?;
        Ok(result)
    }

    /// Add a magnet link and return the torrent ID
    #[allow(dead_code)]
    pub async fn add_magnet(&self, magnet: &str) -> Result<String, ApiError> {
        let url = format!("{}/torrents/addMagnet", RD_API_URL);

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .form(&[("magnet", magnet)])
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ApiError::RealDebridAuth);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ApiError::RealDebrid(format!(
                "Failed to add magnet: {}",
                error_text
            )));
        }

        let result: AddMagnetResponse = response.json().await?;
        Ok(result.id)
    }

    /// Get torrent info
    #[allow(dead_code)]
    pub async fn get_torrent_info(&self, id: &str) -> Result<TorrentInfo, ApiError> {
        let url = format!("{}/torrents/info/{}", RD_API_URL, id);

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

        let info: TorrentInfo = response.json().await?;
        Ok(info)
    }

    /// Select files from a torrent (select all by default)
    #[allow(dead_code)]
    pub async fn select_files(&self, id: &str) -> Result<(), ApiError> {
        let url = format!("{}/torrents/selectFiles/{}", RD_API_URL, id);

        let response = self
            .client
            .post(&url)
            .header("Authorization", self.auth_header())
            .form(&[("files", "all")])
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ApiError::RealDebridAuth);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(ApiError::RealDebrid(format!(
                "Failed to select files: {}",
                error_text
            )));
        }

        Ok(())
    }

    /// Check instant availability for a hash
    #[allow(dead_code)]
    pub async fn check_instant(&self, hash: &str) -> Result<bool, ApiError> {
        let url = format!("{}/torrents/instantAvailability/{}", RD_API_URL, hash);

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
            return Ok(false);
        }

        // The response is a map of hash -> availability info
        // If the hash key exists and has content, it's available
        let text = response.text().await?;
        Ok(text.contains(&hash.to_lowercase()) && text.contains("\"rd\""))
    }
}

#[derive(Debug, Deserialize)]
pub struct RealDebridUser {
    #[allow(dead_code)]
    pub id: i64,
    pub username: String,
    #[allow(dead_code)]
    pub email: String,
    #[allow(dead_code)]
    pub premium: i64,
    #[allow(dead_code)]
    #[serde(rename = "type")]
    pub user_type: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct UnrestrictedLink {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub filename: String,
    #[allow(dead_code)]
    pub filesize: i64,
    pub download: String,
    #[allow(dead_code)]
    #[serde(rename = "streamable")]
    pub is_streamable: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AddMagnetResponse {
    id: String,
    #[allow(dead_code)]
    uri: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TorrentInfo {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub filename: String,
    pub status: String,
    #[allow(dead_code)]
    pub progress: f32,
    pub links: Vec<String>,
}

#[allow(dead_code)]
impl TorrentInfo {
    /// Check if torrent is ready for streaming
    pub fn is_ready(&self) -> bool {
        self.status == "downloaded" && !self.links.is_empty()
    }

    /// Check if torrent is still downloading
    pub fn is_downloading(&self) -> bool {
        self.status == "downloading" || self.status == "queued" || self.status == "waiting_files_selection"
    }
}
