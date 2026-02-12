//! Linux Screen Time Collector
//!
//! Tracks foreground application changes on Linux using X11.

use super::{is_idle, Collector, CollectorHandle, DEFAULT_IDLE_THRESHOLD_SECS};
use crate::db::Database;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// FFI bindings for X11 active window
#[cfg(target_os = "linux")]
mod ffi {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, Window};
    use x11rb::rust_connection::RustConnection;

    /// Get the active window name and class from X11
    pub fn get_frontmost_app() -> Option<(String, String)> {
        let (conn, screen_num) = RustConnection::connect(None).ok()?;
        let screen = &conn.setup().roots[screen_num];
        let root = screen.root;

        // Get _NET_ACTIVE_WINDOW atom
        let active_window_atom = conn
            .intern_atom(false, b"_NET_ACTIVE_WINDOW")
            .ok()?
            .reply()
            .ok()?
            .atom;

        // Get the active window
        let active_window_reply = conn
            .get_property(false, root, active_window_atom, AtomEnum::WINDOW, 0, 1)
            .ok()?
            .reply()
            .ok()?;

        if active_window_reply.value.len() < 4 {
            return None;
        }

        let active_window = u32::from_ne_bytes([
            active_window_reply.value[0],
            active_window_reply.value[1],
            active_window_reply.value[2],
            active_window_reply.value[3],
        ]);

        if active_window == 0 {
            return None;
        }

        // Get _NET_WM_NAME (modern window name)
        let wm_name = get_window_name(&conn, active_window as Window)?;

        // Get WM_CLASS (application class)
        let wm_class = get_window_class(&conn, active_window as Window)?;

        Some((wm_name, wm_class))
    }

    fn get_window_name(conn: &RustConnection, window: Window) -> Option<String> {
        // Try _NET_WM_NAME first (UTF-8)
        let net_wm_name_atom = conn
            .intern_atom(false, b"_NET_WM_NAME")
            .ok()?
            .reply()
            .ok()?
            .atom;
        let utf8_string_atom = conn
            .intern_atom(false, b"UTF8_STRING")
            .ok()?
            .reply()
            .ok()?
            .atom;

        let reply = conn
            .get_property(false, window, net_wm_name_atom, utf8_string_atom, 0, 1024)
            .ok()?
            .reply()
            .ok()?;

        if !reply.value.is_empty() {
            return String::from_utf8(reply.value).ok();
        }

        // Fall back to WM_NAME (legacy)
        let reply = conn
            .get_property(false, window, AtomEnum::WM_NAME, AtomEnum::STRING, 0, 1024)
            .ok()?
            .reply()
            .ok()?;

        if !reply.value.is_empty() {
            return String::from_utf8_lossy(&reply.value).into_owned().into();
        }

        None
    }

    fn get_window_class(conn: &RustConnection, window: Window) -> Option<String> {
        let reply = conn
            .get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 1024)
            .ok()?
            .reply()
            .ok()?;

        if reply.value.is_empty() {
            return None;
        }

        // WM_CLASS is two null-terminated strings: instance name and class name
        // We want the class name (second one)
        let parts: Vec<&[u8]> = reply.value.split(|&b| b == 0).collect();
        if parts.len() >= 2 && !parts[1].is_empty() {
            return String::from_utf8_lossy(parts[1]).into_owned().into();
        } else if !parts[0].is_empty() {
            return String::from_utf8_lossy(parts[0]).into_owned().into();
        }

        None
    }
}

#[cfg(not(target_os = "linux"))]
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
    /// Current foreground app
    current_app: Option<String>,
    /// Current window title (if tracking)
    current_title: Option<String>,
    /// Start time of current session
    session_start: chrono::DateTime<chrono::Utc>,
    /// Accumulated idle time this session
    idle_time_secs: u64,
}

impl SessionState {
    fn new() -> Self {
        Self {
            current_app: None,
            current_title: None,
            session_start: chrono::Utc::now(),
            idle_time_secs: 0,
        }
    }
}

/// Linux Screen Time Collector
///
/// Tracks foreground application and window title on Linux using X11.
pub struct LinuxScreenTimeCollector {
    handle: CollectorHandle,
    config: ScreenTimeConfig,
    db_path: String,
}

impl LinuxScreenTimeCollector {
    /// Create a new Linux screen time collector
    pub fn new(db_path: impl AsRef<Path>, config: ScreenTimeConfig) -> Self {
        Self {
            handle: CollectorHandle::new("linux_screen_time"),
            config,
            db_path: db_path.as_ref().to_string_lossy().to_string(),
        }
    }

    /// Create with default configuration
    pub fn with_defaults(db_path: impl AsRef<Path>) -> Self {
        Self::new(db_path, ScreenTimeConfig::default())
    }
}

impl Collector for LinuxScreenTimeCollector {
    fn name(&self) -> &str {
        self.handle.name()
    }

    fn start(&mut self) -> anyhow::Result<()> {
        let config = self.config.clone();
        let db_path = self.db_path.clone();

        self.handle.start(move |running| {
            let db = match Database::open(&db_path) {
                Ok(db) => db,
                Err(e) => {
                    eprintln!("Failed to open database: {}", e);
                    return;
                }
            };

            let mut state = SessionState::new();
            let poll_duration = Duration::from_millis(config.poll_interval_ms);

            while running.load(Ordering::SeqCst) {
                // Check if user is idle
                if is_idle(config.idle_threshold_secs) {
                    state.idle_time_secs += config.poll_interval_ms / 1000;
                    std::thread::sleep(poll_duration);
                    continue;
                }

                // Get current foreground app
                if let Some((name, bundle_id)) = ffi::get_frontmost_app() {
                    let title = if config.track_window_titles {
                        Some(name.clone())
                    } else {
                        None
                    };

                    // Check if app changed
                    let app_changed = state.current_app.as_ref() != Some(&bundle_id);
                    let title_changed = config.track_window_titles
                        && state.current_title.as_ref() != title.as_ref();

                    if app_changed || title_changed {
                        // Save previous session if there was one
                        if let Some(prev_app) = &state.current_app {
                            let duration = chrono::Utc::now()
                                .signed_duration_since(state.session_start)
                                .num_seconds() as u64;

                            if duration > 0 {
                                let active_time = duration.saturating_sub(state.idle_time_secs);
                                if let Err(e) = db.record_screen_time(
                                    prev_app,
                                    state.current_title.as_deref(),
                                    active_time,
                                    state.idle_time_secs,
                                ) {
                                    eprintln!("Failed to record screen time: {}", e);
                                }
                            }
                        }

                        // Start new session
                        state.current_app = Some(bundle_id);
                        state.current_title = title;
                        state.session_start = chrono::Utc::now();
                        state.idle_time_secs = 0;
                    }
                }

                std::thread::sleep(poll_duration);
            }

            // Save final session on shutdown
            if let Some(app) = &state.current_app {
                let duration = chrono::Utc::now()
                    .signed_duration_since(state.session_start)
                    .num_seconds() as u64;

                if duration > 0 {
                    let active_time = duration.saturating_sub(state.idle_time_secs);
                    let _ = db.record_screen_time(
                        app,
                        state.current_title.as_deref(),
                        active_time,
                        state.idle_time_secs,
                    );
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
        assert_eq!(config.idle_threshold_secs, DEFAULT_IDLE_THRESHOLD_SECS);
        assert!(!config.track_window_titles);
    }

    #[test]
    fn test_linux_screen_time_collector_new() {
        let collector = LinuxScreenTimeCollector::with_defaults("/tmp/test.db");
        assert_eq!(collector.name(), "linux_screen_time");
        assert!(!collector.is_running());
    }

    #[test]
    fn test_session_state_new() {
        let state = SessionState::new();
        assert!(state.current_app.is_none());
        assert!(state.current_title.is_none());
        assert_eq!(state.idle_time_secs, 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_get_frontmost_app_runs() {
        // Just verify it doesn't panic - actual result depends on X11 being available
        let _ = ffi::get_frontmost_app();
    }
}
