//! CSV export functionality

use crate::analytics::{compute_daily_summary, default_categories};
use crate::db::Database;
use anyhow::Result;
use chrono::{Duration, NaiveDate};

#[cfg(test)]
use chrono::Utc;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// Export daily summaries to CSV
///
/// Format: date,total_screen_time_secs,total_screen_time_formatted
pub fn export_daily_csv(db: &Database, start: NaiveDate, end: NaiveDate) -> Result<String> {
    let categories = default_categories();
    let mut csv = String::from("date,total_screen_time_secs,total_screen_time_formatted\n");

    let mut current = start;
    while current <= end {
        let summary = compute_daily_summary(db, current, Some(&categories))?;
        csv.push_str(&format!(
            "{},{},{}\n",
            current,
            summary.total_screen_time_secs,
            summary.format_total_time()
        ));
        current += Duration::days(1);
    }

    Ok(csv)
}

/// Export per-app breakdown to CSV
///
/// Format: date,app_name,bundle_id,duration_secs,duration_formatted
pub fn export_apps_csv(db: &Database, start: NaiveDate, end: NaiveDate) -> Result<String> {
    let categories = default_categories();
    let mut csv = String::from("date,app_name,bundle_id,duration_secs,duration_formatted\n");

    let mut current = start;
    while current <= end {
        let summary = compute_daily_summary(db, current, Some(&categories))?;
        for app in &summary.top_apps {
            let hours = app.seconds / 3600;
            let minutes = (app.seconds % 3600) / 60;
            let formatted = if hours > 0 {
                format!("{}h {}m", hours, minutes)
            } else {
                format!("{}m", minutes)
            };
            // Escape app names that might contain commas
            let escaped_name = escape_csv_field(&app.name);
            csv.push_str(&format!(
                "{},{},{},{},{}\n",
                current, escaped_name, app.bundle_id, app.seconds, formatted
            ));
        }
        current += Duration::days(1);
    }

    Ok(csv)
}

/// Export category breakdown to CSV
///
/// Format: date,category,duration_secs,duration_formatted
pub fn export_categories_csv(db: &Database, start: NaiveDate, end: NaiveDate) -> Result<String> {
    let categories = default_categories();
    let mut csv = String::from("date,category,duration_secs,duration_formatted\n");

    let mut current = start;
    while current <= end {
        let summary = compute_daily_summary(db, current, Some(&categories))?;
        for cat in &summary.category_breakdown {
            let hours = cat.seconds / 3600;
            let minutes = (cat.seconds % 3600) / 60;
            let formatted = if hours > 0 {
                format!("{}h {}m", hours, minutes)
            } else {
                format!("{}m", minutes)
            };
            csv.push_str(&format!(
                "{},{},{},{}\n",
                current, cat.category, cat.seconds, formatted
            ));
        }
        current += Duration::days(1);
    }

    Ok(csv)
}

/// Escape a CSV field (wrap in quotes if contains comma, quote, or newline)
fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// CSV export type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvExportType {
    /// Daily totals
    Daily,
    /// Per-app breakdown
    Apps,
    /// Category breakdown
    Categories,
}

impl std::str::FromStr for CsvExportType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "daily" => Ok(CsvExportType::Daily),
            "apps" => Ok(CsvExportType::Apps),
            "categories" => Ok(CsvExportType::Categories),
            _ => Err(format!("Unknown CSV export type: {}", s)),
        }
    }
}

/// Export data to a CSV file
pub fn export_to_csv_file(
    db: &Database,
    start: NaiveDate,
    end: NaiveDate,
    output_path: &Path,
    export_type: CsvExportType,
) -> Result<()> {
    let csv = match export_type {
        CsvExportType::Daily => export_daily_csv(db, start, end)?,
        CsvExportType::Apps => export_apps_csv(db, start, end)?,
        CsvExportType::Categories => export_categories_csv(db, start, end)?,
    };

    let mut file = File::create(output_path)?;
    file.write_all(csv.as_bytes())?;

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
    fn test_export_daily_csv_empty() {
        let (db, _file) = setup_test_db();
        let today = Utc::now().date_naive();

        let csv = export_daily_csv(&db, today, today).unwrap();

        assert!(csv.starts_with("date,total_screen_time_secs,total_screen_time_formatted\n"));
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 2); // header + 1 day
    }

    #[test]
    fn test_export_daily_csv_multiple_days() {
        let (db, _file) = setup_test_db();
        let today = Utc::now().date_naive();
        let week_ago = today - Duration::days(7);

        let csv = export_daily_csv(&db, week_ago, today).unwrap();

        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), 9); // header + 8 days
    }

    #[test]
    fn test_export_apps_csv_header() {
        let (db, _file) = setup_test_db();
        let today = Utc::now().date_naive();

        let csv = export_apps_csv(&db, today, today).unwrap();

        assert!(csv.starts_with("date,app_name,bundle_id,duration_secs,duration_formatted\n"));
    }

    #[test]
    fn test_export_categories_csv_header() {
        let (db, _file) = setup_test_db();
        let today = Utc::now().date_naive();

        let csv = export_categories_csv(&db, today, today).unwrap();

        assert!(csv.starts_with("date,category,duration_secs,duration_formatted\n"));
    }

    #[test]
    fn test_escape_csv_field_simple() {
        assert_eq!(escape_csv_field("hello"), "hello");
    }

    #[test]
    fn test_escape_csv_field_with_comma() {
        assert_eq!(escape_csv_field("hello, world"), "\"hello, world\"");
    }

    #[test]
    fn test_escape_csv_field_with_quote() {
        assert_eq!(escape_csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn test_csv_export_type_from_str() {
        assert_eq!("daily".parse::<CsvExportType>().unwrap(), CsvExportType::Daily);
        assert_eq!("apps".parse::<CsvExportType>().unwrap(), CsvExportType::Apps);
        assert_eq!("CATEGORIES".parse::<CsvExportType>().unwrap(), CsvExportType::Categories);
        assert!("invalid".parse::<CsvExportType>().is_err());
    }

    #[test]
    fn test_export_to_csv_file() {
        let (db, _db_file) = setup_test_db();
        let output_file = NamedTempFile::new().unwrap();
        let today = Utc::now().date_naive();

        export_to_csv_file(&db, today, today, output_file.path(), CsvExportType::Daily).unwrap();

        let content = std::fs::read_to_string(output_file.path()).unwrap();
        assert!(content.contains("date,total_screen_time_secs"));
    }
}
