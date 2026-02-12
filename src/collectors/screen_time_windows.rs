//! Windows Screen Time Collector
//!
//! Tracks foreground application changes on Windows using Win32 API.

use super::{is_idle, Collector, CollectorHandle, DEFAULT_IDLE_THRESHOLD_SECS};
use crate::db::Database;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::Duration;

/// FFI bindings for Win32 active window
#[cfg(target_os = "windows")]
mod ffi {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowTextW, GetWindowThreadProcessId,
    };
    use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};

    /// Get the foreground window name and process name
    pub fn get_frontmost_app() -> Option<(String, String)> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.0 == 0 {
                return None;
            }

            // Get window title
            let mut title_buf = [0u16; 512];
            let title_len = GetWindowTextW(hwnd, &mut title_buf);
            let title = if title_len > 0 {
                OsString::from_wide(&title_buf[..title_len as usize])
                    .to_string_lossy()
                    .to_string()
            } else {
                String::new()
            };

            // Get process ID
            let mut process_id: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut process_id));

            if process_id == 0 {
                return Some((title, String::from("unknown")));
            }

            // Open process to get executable name
            let process_handle = OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                false,
                process_id,
            );

            let process_name = if let Ok(handle) = process_handle {
                let mut exe_buf = [0u16; 512];
                let exe_len = GetModuleFileNameExW(handle, None, &mut exe_buf);
                if exe_len > 0 {
                    let exe_path = OsString::from_wide(&exe_buf[..exe_len as usize])
                        .to_string_lossy()
                        .to_string();
                    // Extract just the filename
                    exe_path
                        .rsplit('\\')
                        .next()
                        .unwrap_or(&exe_path)
                        .to_string()
                } else {
                    String::from("unknown")
                }
            } else {
                String::from("unknown")
            };

            Some((title, process_name))
        }
    }

    /// Get milliseconds since last user input
    pub fn get_idle_time_ms() -> u32 {
        use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

        unsafe {
            let mut last_input = LASTINPUTINFO {
                cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                dwTime: 0,
            };

            if GetLastInputInfo(&mut last_input).as_bool() {
                let current_tick = windows::Win32::System::SystemInformation::GetTickCount();
                current_tick.wrapping_sub(last_input.dwTime)
            } else {
                0
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod ffi {
    pub fn get_frontmost_app() -> Option<(String, String)> {
        None
    }

    pub fn get_idle_time_ms() -> u32 {
        0
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

/// Windows Screen Time Collector
///
/// Tracks foreground application and window title on Windows using Win32 API.
pub struct WindowsScreenTimeCollector {
    handle: CollectorHandle,
    config: ScreenTimeConfig,
    db_path: String,
}

impl WindowsScreenTimeCollector {
    /// Create a new Windows screen time collector
    pub fn new(db_path: impl AsRef<Path>, config: ScreenTimeConfig) -> Self {
        Self {
            handle: CollectorHandle::new("windows_screen_time"),
            config,
            db_path: db_path.as_ref().to_string_lossy().to_string(),
        }
    }

    /// Create with default configuration
    pub fn with_defaults(db_path: impl AsRef<Path>) -> Self {
        Self::new(db_path, ScreenTimeConfig::default())
    }
}

impl Collector for WindowsScreenTimeCollector {
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
                // Check if user is idle using Windows-specific API
                let idle_ms = ffi::get_idle_time_ms();
                let is_user_idle = idle_ms >= (config.idle_threshold_secs * 1000) as u32;

                if is_user_idle {
                    state.idle_time_secs += config.poll_interval_ms / 1000;
                    std::thread::sleep(poll_duration);
                    continue;
                }

                // Get current foreground app
                if let Some((title, process_name)) = ffi::get_frontmost_app() {
                    let window_title = if config.track_window_titles {
                        Some(title.clone())
                    } else {
                        None
                    };

                    // Check if app changed
                    let app_changed = state.current_app.as_ref() != Some(&process_name);
                    let title_changed = config.track_window_titles
                        && state.current_title.as_ref() != window_title.as_ref();

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
                        state.current_app = Some(process_name);
                        state.current_title = window_title;
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
    fn test_windows_screen_time_collector_new() {
        let collector = WindowsScreenTimeCollector::with_defaults("/tmp/test.db");
        assert_eq!(collector.name(), "windows_screen_time");
        assert!(!collector.is_running());
    }

    #[test]
    fn test_session_state_new() {
        let state = SessionState::new();
        assert!(state.current_app.is_none());
        assert!(state.current_title.is_none());
        assert_eq!(state.idle_time_secs, 0);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_get_frontmost_app_runs() {
        // Just verify it doesn't panic
        let _ = ffi::get_frontmost_app();
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_get_idle_time_runs() {
        let idle_ms = ffi::get_idle_time_ms();
        // Should return something (0 or more)
        assert!(idle_ms >= 0);
    }
}
