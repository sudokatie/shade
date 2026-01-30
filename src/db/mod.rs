//! Database layer for Shade
//!
//! SQLite-based storage for all tracking data.

mod queries;
mod schema;

pub use queries::Database;
pub use schema::{Application, Session};
