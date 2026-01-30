//! TUI dashboard for Shade
//!
//! Interactive terminal interface for viewing analytics.

mod app;
mod run;
mod ui;

pub use app::{App, Tab};
pub use run::run;
