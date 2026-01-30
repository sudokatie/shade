//! JSON export functionality

use crate::analytics::{compute_daily_summary, default_categories, DailySummary};
use crate::db::Database;
use anyhow::Result;
use chrono::{Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Exported session data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionExport {
    pub app_name: String,
    pub bundle_id: String,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration_secs: i64,
    pub ended_idle: bool,
}

/// Full export data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub export_date: String,
    pub date_range: DateRange,
    pub summaries: Vec<DailySummary>,
    pub total_screen_time_secs: i64,
    pub app_count: usize,
}

/// Date range for export
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: String,
    pub end: String,
}

/// Export data for a date range
/// 
/// # Arguments
/// * `db` - Database connection
/// * `start` - Start date (inclusive)
/// * `end` - End date (inclusive)
pub fn export_range(db: &Database, start: NaiveDate, end: NaiveDate) -> Result<ExportData> {
    let categories = default_categories();
    let mut summaries = Vec::new();
    let mut total_secs = 0i64;
    
    // Iterate through each day in range
    let mut current = start;
    while current <= end {
        let summary = compute_daily_summary(db, current, Some(&categories))?;
        total_secs += summary.total_screen_time_secs;
        summaries.push(summary);
        current += Duration::days(1);
    }
    
    // Get unique app count
    let apps = db.get_all_applications()?;
    
    Ok(ExportData {
        export_date: Utc::now().to_rfc3339(),
        date_range: DateRange {
            start: start.to_string(),
            end: end.to_string(),
        },
        summaries,
        total_screen_time_secs: total_secs,
        app_count: apps.len(),
    })
}

/// Export data to a JSON file
/// 
/// # Arguments
/// * `db` - Database connection
/// * `start` - Start date (inclusive)
/// * `end` - End date (inclusive)
/// * `output_path` - Path to write JSON file
pub fn export_to_file(
    db: &Database,
    start: NaiveDate,
    end: NaiveDate,
    output_path: &Path,
) -> Result<()> {
    let data = export_range(db, start, end)?;
    let json = serde_json::to_string_pretty(&data)?;
    
    let mut file = File::create(output_path)?;
    file.write_all(json.as_bytes())?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn setup_test_db() -> (Database, NamedTempFile) {
        let file = NamedTempFile::new().unwrap();
        let db = Database::open(file.path()).unwrap();
        (db, file)
    }

    #[test]
    fn test_export_range_empty() {
        let (db, _file) = setup_test_db();
        let today = Utc::now().date_naive();
        
        let export = export_range(&db, today, today).unwrap();
        
        assert_eq!(export.summaries.len(), 1);
        assert_eq!(export.total_screen_time_secs, 0);
        assert_eq!(export.app_count, 0);
    }

    #[test]
    fn test_export_range_multiple_days() {
        let (db, _file) = setup_test_db();
        let today = Utc::now().date_naive();
        let week_ago = today - Duration::days(7);
        
        let export = export_range(&db, week_ago, today).unwrap();
        
        // Should have 8 days (week_ago to today inclusive)
        assert_eq!(export.summaries.len(), 8);
    }

    #[test]
    fn test_export_to_file() {
        let (db, _db_file) = setup_test_db();
        let output_file = NamedTempFile::new().unwrap();
        let today = Utc::now().date_naive();
        
        export_to_file(&db, today, today, output_file.path()).unwrap();
        
        // Verify file was created and is valid JSON
        let content = std::fs::read_to_string(output_file.path()).unwrap();
        let parsed: ExportData = serde_json::from_str(&content).unwrap();
        
        assert_eq!(parsed.summaries.len(), 1);
    }

    #[test]
    fn test_export_data_serialization() {
        let export = ExportData {
            export_date: "2026-01-30T12:00:00Z".to_string(),
            date_range: DateRange {
                start: "2026-01-30".to_string(),
                end: "2026-01-30".to_string(),
            },
            summaries: vec![],
            total_screen_time_secs: 3600,
            app_count: 5,
        };
        
        let json = serde_json::to_string(&export).unwrap();
        assert!(json.contains("total_screen_time_secs"));
        assert!(json.contains("3600"));
        
        // Verify round-trip
        let parsed: ExportData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_screen_time_secs, 3600);
        assert_eq!(parsed.app_count, 5);
    }
}
