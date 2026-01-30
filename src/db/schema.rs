//! Database schema definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An application being tracked
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    /// Unique ID
    pub id: i64,
    /// Bundle ID (macOS) or executable path
    pub bundle_id: String,
    /// Display name
    pub name: String,
    /// Category (if classified)
    pub category: Option<String>,
    /// When first seen
    pub first_seen: DateTime<Utc>,
}

/// A usage session for an application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique ID
    pub id: i64,
    /// Application ID (foreign key)
    pub application_id: i64,
    /// When the session started
    pub started_at: DateTime<Utc>,
    /// When the session ended (None if still active)
    pub ended_at: Option<DateTime<Utc>>,
    /// Window title (if tracking enabled)
    pub window_title: Option<String>,
    /// Whether session ended due to idle
    pub ended_idle: bool,
}

impl Session {
    /// Calculate session duration in seconds
    pub fn duration_secs(&self) -> i64 {
        match self.ended_at {
            Some(end) => (end - self.started_at).num_seconds(),
            None => (Utc::now() - self.started_at).num_seconds(),
        }
    }
}

/// SQL for creating the database schema
pub const CREATE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS applications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    bundle_id TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    category TEXT,
    first_seen TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    application_id INTEGER NOT NULL REFERENCES applications(id),
    started_at TEXT NOT NULL,
    ended_at TEXT,
    window_title TEXT,
    ended_idle INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at);
CREATE INDEX IF NOT EXISTS idx_sessions_application_id ON sessions(application_id);
CREATE INDEX IF NOT EXISTS idx_applications_bundle_id ON applications(bundle_id);
"#;
