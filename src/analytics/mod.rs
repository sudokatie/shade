//! Analytics and aggregation for Shade
//!
//! Computes summaries, trends, and insights from collected data.

mod categories;
mod daily;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

pub use categories::{
    categorize_apps, default_categories, get_category_for_bundle_id, merge_categories,
};
pub use daily::{compute_daily_summary, compute_today_summary};

/// Daily summary of usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySummary {
    /// The date
    pub date: NaiveDate,
    /// Total screen time in seconds
    pub total_screen_time_secs: i64,
    /// Breakdown by category
    pub category_breakdown: Vec<CategoryTime>,
    /// Top applications
    pub top_apps: Vec<AppTime>,
}

/// Time spent in a category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryTime {
    pub category: String,
    pub seconds: i64,
}

/// Time spent in an application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTime {
    pub name: String,
    pub bundle_id: String,
    pub seconds: i64,
}

impl DailySummary {
    /// Format total screen time as "Xh Ym"
    pub fn format_total_time(&self) -> String {
        let hours = self.total_screen_time_secs / 3600;
        let minutes = (self.total_screen_time_secs % 3600) / 60;
        format!("{}h {}m", hours, minutes)
    }
}
