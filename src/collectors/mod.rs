//! Data collectors for Shade
//!
//! Collectors gather data from various sources (screen time, apps, etc.)
//! and store it in the database. They run in background threads.

mod collector;

pub use collector::{Collector, CollectorHandle};
