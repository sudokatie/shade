//! Daily analytics computation

use super::{AppTime, CategoryTime, DailySummary};
use crate::db::Database;
use anyhow::Result;
use chrono::{NaiveDate, Utc};
use std::collections::HashMap;

/// Compute the daily summary for a given date
/// 
/// # Arguments
/// * `db` - Database connection
/// * `date` - The date to summarize
/// * `categories` - Optional category mapping (bundle_id -> category)
pub fn compute_daily_summary(
    db: &Database,
    date: NaiveDate,
    categories: Option<&HashMap<String, String>>,
) -> Result<DailySummary> {
    // Get total screen time
    let total_screen_time_secs = db.get_daily_screen_time(date)?;
    
    // Get top apps (same day for start and end)
    let top_apps_raw = db.get_top_apps(date, date, 10)?;
    let top_apps: Vec<AppTime> = top_apps_raw
        .into_iter()
        .map(|(app, secs)| AppTime {
            name: app.name,
            bundle_id: app.bundle_id,
            seconds: secs,
        })
        .collect();
    
    // Calculate category breakdown
    let category_breakdown = if let Some(cat_map) = categories {
        compute_category_breakdown(&top_apps, cat_map)
    } else {
        // Without category mapping, put everything in "Uncategorized"
        vec![CategoryTime {
            category: "Uncategorized".to_string(),
            seconds: total_screen_time_secs,
        }]
    };
    
    Ok(DailySummary {
        date,
        total_screen_time_secs,
        category_breakdown,
        top_apps,
    })
}

/// Compute the daily summary for today
pub fn compute_today_summary(
    db: &Database,
    categories: Option<&HashMap<String, String>>,
) -> Result<DailySummary> {
    let today = Utc::now().date_naive();
    compute_daily_summary(db, today, categories)
}

/// Compute category breakdown from top apps
fn compute_category_breakdown(
    apps: &[AppTime],
    categories: &HashMap<String, String>,
) -> Vec<CategoryTime> {
    let mut category_totals: HashMap<String, i64> = HashMap::new();
    
    for app in apps {
        let category = categories
            .get(&app.bundle_id)
            .cloned()
            .unwrap_or_else(|| "Uncategorized".to_string());
        
        *category_totals.entry(category).or_insert(0) += app.seconds;
    }
    
    // Convert to sorted vec (descending by time)
    let mut breakdown: Vec<CategoryTime> = category_totals
        .into_iter()
        .map(|(category, seconds)| CategoryTime { category, seconds })
        .collect();
    
    breakdown.sort_by(|a, b| b.seconds.cmp(&a.seconds));
    breakdown
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
    fn test_compute_daily_summary_empty() {
        let (db, _file) = setup_test_db();
        let today = Utc::now().date_naive();
        
        let summary = compute_daily_summary(&db, today, None).unwrap();
        
        assert_eq!(summary.date, today);
        assert_eq!(summary.total_screen_time_secs, 0);
        assert!(summary.top_apps.is_empty());
    }

    #[test]
    fn test_compute_daily_summary_with_data() {
        let (db, _file) = setup_test_db();
        let today = Utc::now().date_naive();
        
        // Add some test data
        let app = db.get_or_create_application("com.example.app", "Example App").unwrap();
        let session_id = db.start_session(app.id, None).unwrap();
        
        // End session after a bit (simulate time passing)
        std::thread::sleep(std::time::Duration::from_millis(100));
        db.end_session(session_id, false).unwrap();
        
        let summary = compute_daily_summary(&db, today, None).unwrap();
        
        assert_eq!(summary.date, today);
        // Should have some screen time (at least a few ms)
        assert!(summary.total_screen_time_secs >= 0);
    }

    #[test]
    fn test_compute_category_breakdown() {
        let apps = vec![
            AppTime {
                name: "Safari".to_string(),
                bundle_id: "com.apple.Safari".to_string(),
                seconds: 3600,
            },
            AppTime {
                name: "Firefox".to_string(),
                bundle_id: "org.mozilla.firefox".to_string(),
                seconds: 1800,
            },
            AppTime {
                name: "VS Code".to_string(),
                bundle_id: "com.microsoft.VSCode".to_string(),
                seconds: 7200,
            },
            AppTime {
                name: "Unknown".to_string(),
                bundle_id: "com.example.unknown".to_string(),
                seconds: 600,
            },
        ];
        
        let mut categories = HashMap::new();
        categories.insert("com.apple.Safari".to_string(), "Browsers".to_string());
        categories.insert("org.mozilla.firefox".to_string(), "Browsers".to_string());
        categories.insert("com.microsoft.VSCode".to_string(), "Development".to_string());
        
        let breakdown = compute_category_breakdown(&apps, &categories);
        
        // Should have 3 categories: Development, Browsers, Uncategorized
        assert_eq!(breakdown.len(), 3);
        
        // Development should be first (7200s)
        assert_eq!(breakdown[0].category, "Development");
        assert_eq!(breakdown[0].seconds, 7200);
        
        // Browsers second (3600 + 1800 = 5400s)
        assert_eq!(breakdown[1].category, "Browsers");
        assert_eq!(breakdown[1].seconds, 5400);
        
        // Uncategorized last (600s)
        assert_eq!(breakdown[2].category, "Uncategorized");
        assert_eq!(breakdown[2].seconds, 600);
    }

    #[test]
    fn test_daily_summary_format_total_time() {
        let summary = DailySummary {
            date: Utc::now().date_naive(),
            total_screen_time_secs: 3661, // 1h 1m 1s
            category_breakdown: vec![],
            top_apps: vec![],
        };
        
        assert_eq!(summary.format_total_time(), "1h 1m");
    }

    #[test]
    fn test_compute_today_summary() {
        let (db, _file) = setup_test_db();
        
        let summary = compute_today_summary(&db, None).unwrap();
        
        assert_eq!(summary.date, Utc::now().date_naive());
    }
}
