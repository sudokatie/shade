//! Database layer for Shade
//!
//! SQLite-based storage for all tracking data.

mod schema;
mod queries;

pub use schema::{Application, Session};
pub use queries::Database;
