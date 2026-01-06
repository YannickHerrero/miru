//! Watch history tracking module
//!
//! Provides persistent storage for tracking watched media using SQLite.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};

use crate::api::MediaType;

/// Get the database file path (~/.config/miru/history.db)
pub fn db_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("miru")
        .join("history.db")
}

/// A watched item record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchedItem {
    /// Unique ID in database
    pub id: i64,
    /// TMDB ID of the media
    pub tmdb_id: i32,
    /// Type of media (Movie or TvShow)
    pub media_type: MediaType,
    /// Title of the media
    pub title: String,
    /// Season number (0 for movies)
    pub season: u32,
    /// Episode number (0 for movies)
    pub episode: u32,
    /// Episode title (if available)
    pub episode_title: Option<String>,
    /// Cover image URL
    pub cover_image: Option<String>,
    /// When this was watched
    pub watched_at: DateTime<Utc>,
}

impl WatchedItem {
    /// Get a display string for the episode (e.g., "S01E05" or just the movie title)
    pub fn episode_display(&self) -> String {
        if self.media_type == MediaType::Movie {
            String::new()
        } else {
            format!("S{:02}E{:02}", self.season, self.episode)
        }
    }

    /// Get a display string for when this was watched
    pub fn watched_at_display(&self) -> String {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.watched_at);

        if duration.num_minutes() < 1 {
            "Just now".to_string()
        } else if duration.num_hours() < 1 {
            format!("{}m ago", duration.num_minutes())
        } else if duration.num_days() < 1 {
            format!("{}h ago", duration.num_hours())
        } else if duration.num_days() < 7 {
            format!("{}d ago", duration.num_days())
        } else {
            self.watched_at.format("%b %d").to_string()
        }
    }
}

/// Watch history database manager
pub struct WatchHistory {
    conn: Connection,
}

impl WatchHistory {
    /// Open or create the watch history database
    pub fn open() -> SqliteResult<Self> {
        let path = db_path();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&path)?;

        let history = Self { conn };
        history.init_schema()?;

        Ok(history)
    }

    /// Initialize database schema
    fn init_schema(&self) -> SqliteResult<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS watched (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tmdb_id INTEGER NOT NULL,
                media_type TEXT NOT NULL,
                title TEXT NOT NULL,
                season INTEGER NOT NULL DEFAULT 0,
                episode INTEGER NOT NULL DEFAULT 0,
                episode_title TEXT,
                cover_image TEXT,
                watched_at TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(tmdb_id, media_type, season, episode)
            )",
            [],
        )?;

        // Index for fast recent queries
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_watched_at ON watched(watched_at DESC)",
            [],
        )?;

        Ok(())
    }

    /// Record a watched item (insert or update timestamp if already exists)
    #[allow(clippy::too_many_arguments)]
    pub fn mark_watched(
        &self,
        tmdb_id: i32,
        media_type: MediaType,
        title: &str,
        season: u32,
        episode: u32,
        episode_title: Option<&str>,
        cover_image: Option<&str>,
    ) -> SqliteResult<()> {
        let media_type_str = match media_type {
            MediaType::Movie => "movie",
            MediaType::TvShow => "tvshow",
        };

        self.conn.execute(
            "INSERT INTO watched (tmdb_id, media_type, title, season, episode, episode_title, cover_image, watched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'))
             ON CONFLICT(tmdb_id, media_type, season, episode) DO UPDATE SET
                 title = excluded.title,
                 episode_title = excluded.episode_title,
                 cover_image = excluded.cover_image,
                 watched_at = datetime('now')",
            params![tmdb_id, media_type_str, title, season, episode, episode_title, cover_image],
        )?;

        Ok(())
    }

    /// Remove a watched item
    pub fn mark_unwatched(
        &self,
        tmdb_id: i32,
        media_type: MediaType,
        season: u32,
        episode: u32,
    ) -> SqliteResult<()> {
        let media_type_str = match media_type {
            MediaType::Movie => "movie",
            MediaType::TvShow => "tvshow",
        };

        self.conn.execute(
            "DELETE FROM watched WHERE tmdb_id = ?1 AND media_type = ?2 AND season = ?3 AND episode = ?4",
            params![tmdb_id, media_type_str, season, episode],
        )?;

        Ok(())
    }

    /// Check if an episode/movie is watched
    #[allow(dead_code)]
    pub fn is_watched(
        &self,
        tmdb_id: i32,
        media_type: MediaType,
        season: u32,
        episode: u32,
    ) -> bool {
        let media_type_str = match media_type {
            MediaType::Movie => "movie",
            MediaType::TvShow => "tvshow",
        };

        self.conn
            .query_row(
                "SELECT 1 FROM watched WHERE tmdb_id = ?1 AND media_type = ?2 AND season = ?3 AND episode = ?4",
                params![tmdb_id, media_type_str, season, episode],
                |_| Ok(()),
            )
            .is_ok()
    }

    /// Get watched episode count for a season
    pub fn watched_episode_count(&self, tmdb_id: i32, season: u32) -> u32 {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM watched WHERE tmdb_id = ?1 AND media_type = 'tvshow' AND season = ?2",
                params![tmdb_id, season],
                |row| row.get::<_, u32>(0),
            )
            .unwrap_or(0)
    }

    /// Get recent watch history (most recent first)
    #[allow(dead_code)]
    pub fn get_recent(&self, limit: usize) -> Vec<WatchedItem> {
        let mut stmt = match self.conn.prepare(
            "SELECT id, tmdb_id, media_type, title, season, episode, episode_title, cover_image, watched_at
             FROM watched
             ORDER BY watched_at DESC
             LIMIT ?1",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Vec::new(),
        };

        let rows = match stmt.query_map(params![limit as i64], |row| {
            let media_type_str: String = row.get(2)?;
            let media_type = match media_type_str.as_str() {
                "movie" => MediaType::Movie,
                _ => MediaType::TvShow,
            };

            let watched_at_str: String = row.get(8)?;
            let watched_at = DateTime::parse_from_rfc3339(&watched_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| {
                    // Try parsing SQLite datetime format
                    chrono::NaiveDateTime::parse_from_str(&watched_at_str, "%Y-%m-%d %H:%M:%S")
                        .map(|dt| dt.and_utc())
                        .unwrap_or_else(|_| Utc::now())
                });

            Ok(WatchedItem {
                id: row.get(0)?,
                tmdb_id: row.get(1)?,
                media_type,
                title: row.get(3)?,
                season: row.get(4)?,
                episode: row.get(5)?,
                episode_title: row.get(6)?,
                cover_image: row.get(7)?,
                watched_at,
            })
        }) {
            Ok(rows) => rows,
            Err(_) => return Vec::new(),
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Get watched episode numbers for a specific season
    pub fn get_watched_episodes(
        &self,
        tmdb_id: i32,
        season: u32,
    ) -> std::collections::HashSet<u32> {
        let mut stmt = match self.conn.prepare(
            "SELECT episode FROM watched 
             WHERE tmdb_id = ?1 AND media_type = 'tvshow' AND season = ?2",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return std::collections::HashSet::new(),
        };

        let rows = match stmt.query_map(params![tmdb_id, season], |row| row.get::<_, u32>(0)) {
            Ok(rows) => rows,
            Err(_) => return std::collections::HashSet::new(),
        };

        rows.filter_map(|r| r.ok()).collect()
    }

    /// Get unique shows/movies from history (for "continue watching" feature)
    /// Returns the most recent watch entry for each unique media item
    pub fn get_recent_media(&self, limit: usize) -> Vec<WatchedItem> {
        let mut stmt = match self.conn.prepare(
            "SELECT id, tmdb_id, media_type, title, season, episode, episode_title, cover_image, watched_at
             FROM watched w1
             WHERE watched_at = (
                 SELECT MAX(watched_at) FROM watched w2 
                 WHERE w2.tmdb_id = w1.tmdb_id AND w2.media_type = w1.media_type
             )
             ORDER BY watched_at DESC
             LIMIT ?1",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Vec::new(),
        };

        let rows = match stmt.query_map(params![limit as i64], |row| {
            let media_type_str: String = row.get(2)?;
            let media_type = match media_type_str.as_str() {
                "movie" => MediaType::Movie,
                _ => MediaType::TvShow,
            };

            let watched_at_str: String = row.get(8)?;
            let watched_at = DateTime::parse_from_rfc3339(&watched_at_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(&watched_at_str, "%Y-%m-%d %H:%M:%S")
                        .map(|dt| dt.and_utc())
                        .unwrap_or_else(|_| Utc::now())
                });

            Ok(WatchedItem {
                id: row.get(0)?,
                tmdb_id: row.get(1)?,
                media_type,
                title: row.get(3)?,
                season: row.get(4)?,
                episode: row.get(5)?,
                episode_title: row.get(6)?,
                cover_image: row.get(7)?,
                watched_at,
            })
        }) {
            Ok(rows) => rows,
            Err(_) => return Vec::new(),
        };

        rows.filter_map(|r| r.ok()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_db() -> WatchHistory {
        // Use in-memory database for tests
        let conn = Connection::open_in_memory().unwrap();
        let history = WatchHistory { conn };
        history.init_schema().unwrap();
        history
    }

    #[test]
    fn test_mark_watched() {
        let history = create_test_db();

        history
            .mark_watched(
                12345,
                MediaType::TvShow,
                "Test Show",
                1,
                5,
                Some("Episode Title"),
                None,
            )
            .unwrap();

        assert!(history.is_watched(12345, MediaType::TvShow, 1, 5));
        assert!(!history.is_watched(12345, MediaType::TvShow, 1, 6));
    }

    #[test]
    fn test_mark_unwatched() {
        let history = create_test_db();

        history
            .mark_watched(12345, MediaType::TvShow, "Test Show", 1, 5, None, None)
            .unwrap();
        assert!(history.is_watched(12345, MediaType::TvShow, 1, 5));

        history
            .mark_unwatched(12345, MediaType::TvShow, 1, 5)
            .unwrap();
        assert!(!history.is_watched(12345, MediaType::TvShow, 1, 5));
    }

    #[test]
    fn test_watched_count() {
        let history = create_test_db();

        history
            .mark_watched(12345, MediaType::TvShow, "Test Show", 1, 1, None, None)
            .unwrap();
        history
            .mark_watched(12345, MediaType::TvShow, "Test Show", 1, 2, None, None)
            .unwrap();
        history
            .mark_watched(12345, MediaType::TvShow, "Test Show", 1, 3, None, None)
            .unwrap();

        assert_eq!(history.watched_episode_count(12345, 1), 3);
        assert_eq!(history.watched_episode_count(12345, 2), 0);
    }
}
