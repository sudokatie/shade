//! Data collectors for Shade
//!
//! Collectors gather data from various sources (screen time, apps, etc.)
//! and store it in the database. They run in background threads.

mod collector;
mod idle;
mod screen_time_macos;

pub use collector::{Collector, CollectorHandle};
pub use idle::{is_idle, seconds_since_last_input, DEFAULT_IDLE_THRESHOLD_SECS};
pub use screen_time_macos::{MacOSScreenTimeCollector, ScreenTimeConfig};
