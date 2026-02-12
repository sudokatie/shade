//! Data collectors for Shade
//!
//! Collectors gather data from various sources (screen time, apps, etc.)
//! and store it in the database. They run in background threads.

mod collector;
mod idle;

#[cfg(target_os = "macos")]
mod screen_time_macos;

#[cfg(target_os = "linux")]
mod screen_time_linux;

#[cfg(target_os = "windows")]
mod screen_time_windows;

pub use collector::{Collector, CollectorHandle};
pub use idle::{is_idle, seconds_since_last_input, DEFAULT_IDLE_THRESHOLD_SECS};

#[cfg(target_os = "macos")]
pub use screen_time_macos::{MacOSScreenTimeCollector, ScreenTimeConfig};

#[cfg(target_os = "linux")]
pub use screen_time_linux::{LinuxScreenTimeCollector, ScreenTimeConfig};

#[cfg(target_os = "windows")]
pub use screen_time_windows::{WindowsScreenTimeCollector, ScreenTimeConfig};
