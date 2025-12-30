use serde::{Deserialize, Serialize};

/// Type of media content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    Anime,
    Movie,
    TvShow,
}

impl MediaType {
    /// Get a display label for the media type
    pub fn label(&self) -> &'static str {
        match self {
            MediaType::Anime => "Anime",
            MediaType::Movie => "Movie",
            MediaType::TvShow => "TV",
        }
    }
}

/// Source of the media data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MediaSource {
    AniList {
        id: i32,
        id_mal: Option<i32>,
    },
    Tmdb {
        id: i32,
    },
}

/// Unified media structure for all content types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Media {
    /// Type of media (Anime, Movie, TvShow)
    pub media_type: MediaType,
    /// Source of the media data
    pub source: MediaSource,
    /// Primary display title
    pub title: String,
    /// Original/alternative title
    pub title_original: Option<String>,
    /// IMDB ID if known directly (TMDB provides this)
    pub imdb_id: Option<String>,
    /// Release year
    pub year: Option<i32>,
    /// Score (0.0-10.0)
    pub score: Option<f32>,
    /// Number of episodes (for anime/tv shows)
    pub episodes: Option<i32>,
    /// Number of seasons (for tv shows)
    pub seasons: Option<i32>,
    /// Cover image URL
    pub cover_image: Option<String>,
    /// Episode titles (if available)
    pub episode_titles: Vec<String>,
    /// Description/synopsis of the media
    pub description: Option<String>,
    /// Status (e.g., "Finished", "Airing", "Released")
    pub status: Option<String>,
    /// Format (e.g., "TV", "Movie", "OVA")
    pub format: Option<String>,
    /// Genres
    pub genres: Vec<String>,
}

impl Media {
    /// Get the best display title
    pub fn display_title(&self) -> &str {
        &self.title
    }

    /// Get the TMDB ID if available
    pub fn tmdb_id(&self) -> Option<i32> {
        match &self.source {
            MediaSource::Tmdb { id } => Some(*id),
            _ => None,
        }
    }

    /// Get episode list (generated from episode count)
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

/// Episode data structure
#[derive(Debug, Clone)]
pub struct Episode {
    pub number: u32,
    pub title: String,
}

/// Season data structure (for TV shows)
#[derive(Debug, Clone)]
pub struct Season {
    pub number: u32,
    pub episode_count: u32,
}

impl Season {
    /// Get episode list for this season
    pub fn get_episodes(&self) -> Vec<Episode> {
        (1..=self.episode_count)
            .map(|n| Episode {
                number: n,
                title: format!("Episode {}", n),
            })
            .collect()
    }
}
