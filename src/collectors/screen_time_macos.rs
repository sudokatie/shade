//! macOS Screen Time Collector
//!
//! Tracks foreground application changes on macOS using polling.

use super::{Collector, CollectorHandle, is_idle, DEFAULT_IDLE_THRESHOLD_SECS};
use crate::db::Database;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// FFI bindings for NSWorkspace
#[cfg(target_os = "macos")]
mod ffi {
    /// Get the bundle ID of the frontmost application
    /// 
    /// Returns None if unable to determine.
    pub fn get_frontmost_app() -> Option<(String, String)> {
        // Use AppleScript as a reliable cross-version approach
        use std::process::Command;
        
        let output = Command::new("osascript")
            .args([
                "-e",
                r#"tell application "System Events"
                    set frontApp to first application process whose frontmost is true
                    set appName to name of frontApp
                    set bundleId to bundle identifier of frontApp
                    return appName & "|" & bundleId
                end tell"#,
            ])
            .output()
            .ok()?;
        
        if !output.status.success() {
            return None;
        }
        
        let result = String::from_utf8_lossy(&output.stdout);
        let result = result.trim();
        let parts: Vec<&str> = result.splitn(2, '|').collect();
        
        if parts.len() == 2 {
            Some((parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod ffi {
    pub fn get_frontmost_app() -> Option<(String, String)> {
        None
    }
}

/// Configuration for the screen time collector
#[derive(Debug, Clone)]
pub struct ScreenTimeConfig {
    /// Polling interval in milliseconds
    pub poll_interval_ms: u64,
    /// Idle threshold in seconds
    pub idle_threshold_secs: u64,
    /// Whether to track window titles (privacy-sensitive)
    pub track_window_titles: bool,
}

impl Default for ScreenTimeConfig {
    fn default() -> Self {
        Self {
            poll_interval_ms: 1000,
            idle_threshold_secs: DEFAULT_IDLE_THRESHOLD_SECS,
            track_window_titles: false,
        }
    }
}

/// State for tracking the current session
#[derive(Debug)]
struct SessionState {
    /// Current application bundle ID
    bundle_id: String,
    /// Current application name
    app_name: String,
    /// Session ID in database
    session_id: Option<i64>,
    /// Whether currently idle
    is_idle: bool,
}

/// macOS Screen Time Collector
/// 
/// Tracks which application is in the foreground and logs sessions
/// to the database.
pub struct MacOSScreenTimeCollector {
    handle: CollectorHandle,
    config: ScreenTimeConfig,
    db_path: String,
}

impl MacOSScreenTimeCollector {
    /// Create a new screen time collector
    pub fn new(db_path: impl Into<String>, config: ScreenTimeConfig) -> Self {
        Self {
            handle: CollectorHandle::new("screen_time_macos"),
            config,
            db_path: db_path.into(),
        }
    }
    
    /// Create with default config
    pub fn with_defaults(db_path: impl Into<String>) -> Self {
        Self::new(db_path, ScreenTimeConfig::default())
    }
}

impl Collector for MacOSScreenTimeCollector {
    fn name(&self) -> &str {
        self.handle.name()
    }
    
    fn start(&mut self) -> anyhow::Result<()> {
        let config = self.config.clone();
        let db_path = self.db_path.clone();
        
        self.handle.start(move |running| {
            // Open database connection for this thread
            let db = match Database::open(Path::new(&db_path)) {
                Ok(db) => db,
                Err(e) => {
                    eprintln!("Failed to open database: {}", e);
                    return;
                }
            };
            
            let mut state: Option<SessionState> = None;
            
            while running.load(Ordering::SeqCst) {
                // Check for idle
                let currently_idle = is_idle(config.idle_threshold_secs);
                
                // Get frontmost app
                let current_app = ffi::get_frontmost_app();
                
                match (&mut state, current_app, currently_idle) {
                    // No previous state, got an app, not idle -> start session
                    (None, Some((name, bundle_id)), false) => {
                        if let Ok(app) = db.get_or_create_application(&bundle_id, &name) {
                            if let Ok(session_id) = db.start_session(app.id, None) {
                                state = Some(SessionState {
                                    bundle_id,
                                    app_name: name,
                                    session_id: Some(session_id),
                                    is_idle: false,
                                });
                            }
                        }
                    }
                    
                    // Have state, app changed -> end old session, start new
                    (Some(s), Some((name, bundle_id)), false) if s.bundle_id != bundle_id => {
                        // End current session
                        if let Some(session_id) = s.session_id {
                            let _ = db.end_session(session_id, false);
                        }
                        
                        // Start new session
                        if let Ok(app) = db.get_or_create_application(&bundle_id, &name) {
                            if let Ok(session_id) = db.start_session(app.id, None) {
                                s.bundle_id = bundle_id;
                                s.app_name = name;
                                s.session_id = Some(session_id);
                                s.is_idle = false;
                            }
                        }
                    }
                    
                    // Have state, became idle -> end session as idle
                    (Some(s), _, true) if !s.is_idle => {
                        if let Some(session_id) = s.session_id {
                            let _ = db.end_session(session_id, true);
                            s.session_id = None;
                        }
                        s.is_idle = true;
                    }
                    
                    // Was idle, no longer idle -> start new session
                    (Some(s), Some((name, bundle_id)), false) if s.is_idle => {
                        if let Ok(app) = db.get_or_create_application(&bundle_id, &name) {
                            if let Ok(session_id) = db.start_session(app.id, None) {
                                s.bundle_id = bundle_id;
                                s.app_name = name;
                                s.session_id = Some(session_id);
                                s.is_idle = false;
                            }
                        }
                    }
                    
                    // No change, continue
                    _ => {}
                }
                
                std::thread::sleep(Duration::from_millis(config.poll_interval_ms));
            }
            
            // Cleanup: end any active session
            if let Some(s) = state {
                if let Some(session_id) = s.session_id {
                    let _ = db.end_session(session_id, false);
                }
            }
        })
    }
    
    fn stop(&mut self) {
        self.handle.stop();
    }
    
    fn is_running(&self) -> bool {
        self.handle.is_running()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_time_config_default() {
        let config = ScreenTimeConfig::default();
        assert_eq!(config.poll_interval_ms, 1000);
        assert_eq!(config.idle_threshold_secs, 300);
        assert!(!config.track_window_titles);
    }

    #[test]
    #[ignore] // AppleScript can hang in test environments
    fn test_get_frontmost_app() {
        // This test just verifies the function doesn't panic
        // Actual result depends on system state
        let result = ffi::get_frontmost_app();
        // On macOS, should get some result; on other platforms, None
        #[cfg(target_os = "macos")]
        {
            // Should return Some on macOS with an active desktop
            // But in CI or headless, might be None
            let _ = result;
        }
        #[cfg(not(target_os = "macos"))]
        {
            assert!(result.is_none());
        }
    }

    #[test]
    fn test_collector_creation() {
        let collector = MacOSScreenTimeCollector::with_defaults(":memory:");
        assert_eq!(collector.name(), "screen_time_macos");
        assert!(!collector.is_running());
    }
}
