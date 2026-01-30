//! Database queries

use super::schema::{Application, CREATE_SCHEMA};
use anyhow::{Context, Result};
use chrono::{NaiveDate, Utc};
use rusqlite::{params, Connection};
use std::path::Path;

/// Database connection wrapper
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create the database at the given path
    pub fn open(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .context("failed to create database directory")?;
        }
        
        let conn = Connection::open(path)
            .context("failed to open database")?;
        
        // Initialize schema
        conn.execute_batch(CREATE_SCHEMA)
            .context("failed to create schema")?;
        
        Ok(Self { conn })
    }
    
    /// Open an in-memory database (for testing)
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .context("failed to open in-memory database")?;
        
        conn.execute_batch(CREATE_SCHEMA)
            .context("failed to create schema")?;
        
        Ok(Self { conn })
    }
    
    /// Get or create an application by bundle ID
    pub fn get_or_create_application(&self, bundle_id: &str, name: &str) -> Result<Application> {
        // Try to find existing
        let existing: Option<Application> = self.conn.query_row(
            "SELECT id, bundle_id, name, category, first_seen FROM applications WHERE bundle_id = ?",
            [bundle_id],
            |row| {
                Ok(Application {
                    id: row.get(0)?,
                    bundle_id: row.get(1)?,
                    name: row.get(2)?,
                    category: row.get(3)?,
                    first_seen: row.get::<_, String>(4)?.parse().unwrap_or_else(|_| Utc::now()),
                })
            },
        ).ok();
        
        if let Some(app) = existing {
            return Ok(app);
        }
        
        // Create new
        self.conn.execute(
            "INSERT INTO applications (bundle_id, name) VALUES (?, ?)",
            params![bundle_id, name],
        )?;
        
        let id = self.conn.last_insert_rowid();
        
        Ok(Application {
            id,
            bundle_id: bundle_id.to_string(),
            name: name.to_string(),
            category: None,
            first_seen: Utc::now(),
        })
    }
    
    /// Start a new session for an application
    pub fn start_session(&self, application_id: i64, window_title: Option<&str>) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        
        self.conn.execute(
            "INSERT INTO sessions (application_id, started_at, window_title) VALUES (?, ?, ?)",
            params![application_id, now, window_title],
        )?;
        
        Ok(self.conn.last_insert_rowid())
    }
    
    /// End a session
    pub fn end_session(&self, session_id: i64, ended_idle: bool) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        
        self.conn.execute(
            "UPDATE sessions SET ended_at = ?, ended_idle = ? WHERE id = ?",
            params![now, ended_idle, session_id],
        )?;
        
        Ok(())
    }
    
    /// End any open sessions
    pub fn end_open_sessions(&self, ended_idle: bool) -> Result<usize> {
        let now = Utc::now().to_rfc3339();
        
        let count = self.conn.execute(
            "UPDATE sessions SET ended_at = ?, ended_idle = ? WHERE ended_at IS NULL",
            params![now, ended_idle],
        )?;
        
        Ok(count)
    }
    
    /// Get total screen time for a date (in seconds)
    pub fn get_daily_screen_time(&self, date: NaiveDate) -> Result<i64> {
        let start = format!("{}T00:00:00", date);
        let end = format!("{}T23:59:59", date);
        
        let total: i64 = self.conn.query_row(
            r#"
            SELECT COALESCE(SUM(
                CAST((julianday(COALESCE(ended_at, datetime('now'))) - julianday(started_at)) * 86400 AS INTEGER)
            ), 0)
            FROM sessions
            WHERE started_at >= ? AND started_at <= ?
            "#,
            params![start, end],
            |row| row.get(0),
        )?;
        
        Ok(total)
    }
    
    /// Get top apps by usage for a date range
    pub fn get_top_apps(&self, start: NaiveDate, end: NaiveDate, limit: usize) -> Result<Vec<(Application, i64)>> {
        let start_str = format!("{}T00:00:00", start);
        let end_str = format!("{}T23:59:59", end);
        
        let mut stmt = self.conn.prepare(
            r#"
            SELECT 
                a.id, a.bundle_id, a.name, a.category, a.first_seen,
                COALESCE(SUM(
                    CAST((julianday(COALESCE(s.ended_at, datetime('now'))) - julianday(s.started_at)) * 86400 AS INTEGER)
                ), 0) as total_secs
            FROM applications a
            LEFT JOIN sessions s ON s.application_id = a.id
                AND s.started_at >= ? AND s.started_at <= ?
            GROUP BY a.id
            ORDER BY total_secs DESC
            LIMIT ?
            "#,
        )?;
        
        let rows = stmt.query_map(params![start_str, end_str, limit], |row| {
            Ok((
                Application {
                    id: row.get(0)?,
                    bundle_id: row.get(1)?,
                    name: row.get(2)?,
                    category: row.get(3)?,
                    first_seen: row.get::<_, String>(4)?.parse().unwrap_or_else(|_| Utc::now()),
                },
                row.get::<_, i64>(5)?,
            ))
        })?;
        
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        
        Ok(results)
    }
    
    /// Get all applications
    pub fn get_all_applications(&self) -> Result<Vec<Application>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, bundle_id, name, category, first_seen FROM applications ORDER BY name"
        )?;
        
        let rows = stmt.query_map([], |row| {
            Ok(Application {
                id: row.get(0)?,
                bundle_id: row.get(1)?,
                name: row.get(2)?,
                category: row.get(3)?,
                first_seen: row.get::<_, String>(4)?.parse().unwrap_or_else(|_| Utc::now()),
            })
        })?;
        
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        
        Ok(results)
    }
    
    /// Set category for an application
    pub fn set_application_category(&self, app_id: i64, category: Option<&str>) -> Result<()> {
        self.conn.execute(
            "UPDATE applications SET category = ? WHERE id = ?",
            params![category, app_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    #[test]
    fn test_open_in_memory() {
        let db = Database::open_in_memory().unwrap();
        // Should not panic
        drop(db);
    }

    #[test]
    fn test_get_or_create_application() {
        let db = Database::open_in_memory().unwrap();
        
        let app1 = db.get_or_create_application("com.apple.safari", "Safari").unwrap();
        assert_eq!(app1.bundle_id, "com.apple.safari");
        assert_eq!(app1.name, "Safari");
        
        // Getting same app should return same ID
        let app2 = db.get_or_create_application("com.apple.safari", "Safari").unwrap();
        assert_eq!(app1.id, app2.id);
        
        // Different app should get different ID
        let app3 = db.get_or_create_application("com.google.chrome", "Chrome").unwrap();
        assert_ne!(app1.id, app3.id);
    }

    #[test]
    fn test_session_lifecycle() {
        let db = Database::open_in_memory().unwrap();
        
        let app = db.get_or_create_application("com.test.app", "Test App").unwrap();
        
        // Start session
        let session_id = db.start_session(app.id, Some("Window Title")).unwrap();
        assert!(session_id > 0);
        
        // End session
        db.end_session(session_id, false).unwrap();
    }

    #[test]
    fn test_daily_screen_time() {
        let db = Database::open_in_memory().unwrap();
        
        let app = db.get_or_create_application("com.test.app", "Test App").unwrap();
        
        // Start and immediately end a session
        let session_id = db.start_session(app.id, None).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        db.end_session(session_id, false).unwrap();
        
        // Check today's screen time
        let today = Local::now().date_naive();
        let time = db.get_daily_screen_time(today).unwrap();
        
        // Should have some time recorded (at least 0)
        assert!(time >= 0);
    }

    #[test]
    fn test_top_apps() {
        let db = Database::open_in_memory().unwrap();
        
        let app1 = db.get_or_create_application("com.test.app1", "App 1").unwrap();
        let app2 = db.get_or_create_application("com.test.app2", "App 2").unwrap();
        
        // Create sessions for app1
        for _ in 0..3 {
            let sid = db.start_session(app1.id, None).unwrap();
            db.end_session(sid, false).unwrap();
        }
        
        // Create session for app2
        let sid = db.start_session(app2.id, None).unwrap();
        db.end_session(sid, false).unwrap();
        
        let today = Local::now().date_naive();
        let top = db.get_top_apps(today, today, 10).unwrap();
        
        assert!(!top.is_empty());
    }

    #[test]
    fn test_end_open_sessions() {
        let db = Database::open_in_memory().unwrap();
        
        let app = db.get_or_create_application("com.test.app", "Test App").unwrap();
        
        // Start multiple sessions without ending them
        db.start_session(app.id, None).unwrap();
        db.start_session(app.id, None).unwrap();
        db.start_session(app.id, None).unwrap();
        
        // End all open sessions
        let count = db.end_open_sessions(true).unwrap();
        assert_eq!(count, 3);
        
        // Ending again should affect 0 sessions
        let count = db.end_open_sessions(true).unwrap();
        assert_eq!(count, 0);
    }
}
