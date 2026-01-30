//! TUI Application state and logic

use crate::analytics::{compute_daily_summary, default_categories, DailySummary};
use crate::db::Database;
use anyhow::Result;
use chrono::Utc;
use std::path::Path;

/// Application state for the TUI
pub struct App {
    /// Whether the app should quit
    pub should_quit: bool,
    /// Current view tab
    pub current_tab: Tab,
    /// Today's summary (cached)
    pub today_summary: Option<DailySummary>,
    /// Selected item index in lists
    pub selected_index: usize,
    /// Error message to display
    pub error_message: Option<String>,
    /// Database path
    db_path: String,
}

/// Available tabs/views
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Today,
    Apps,
    Categories,
}

impl Tab {
    pub fn next(&self) -> Self {
        match self {
            Tab::Today => Tab::Apps,
            Tab::Apps => Tab::Categories,
            Tab::Categories => Tab::Today,
        }
    }
    
    pub fn prev(&self) -> Self {
        match self {
            Tab::Today => Tab::Categories,
            Tab::Apps => Tab::Today,
            Tab::Categories => Tab::Apps,
        }
    }
    
    pub fn title(&self) -> &'static str {
        match self {
            Tab::Today => "Today",
            Tab::Apps => "Top Apps",
            Tab::Categories => "Categories",
        }
    }
}

impl App {
    /// Create a new app instance
    pub fn new(db_path: impl Into<String>) -> Self {
        Self {
            should_quit: false,
            current_tab: Tab::Today,
            today_summary: None,
            selected_index: 0,
            error_message: None,
            db_path: db_path.into(),
        }
    }
    
    /// Load/refresh data from database
    pub fn refresh_data(&mut self) -> Result<()> {
        let db = Database::open(Path::new(&self.db_path))?;
        let today = Utc::now().date_naive();
        let categories = default_categories();
        
        self.today_summary = Some(compute_daily_summary(&db, today, Some(&categories))?);
        self.error_message = None;
        
        Ok(())
    }
    
    /// Handle quit request
    pub fn quit(&mut self) {
        self.should_quit = true;
    }
    
    /// Switch to next tab
    pub fn next_tab(&mut self) {
        self.current_tab = self.current_tab.next();
        self.selected_index = 0;
    }
    
    /// Switch to previous tab
    pub fn prev_tab(&mut self) {
        self.current_tab = self.current_tab.prev();
        self.selected_index = 0;
    }
    
    /// Move selection down
    pub fn select_next(&mut self) {
        let max = self.max_selection_index();
        if self.selected_index < max {
            self.selected_index += 1;
        }
    }
    
    /// Move selection up
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }
    
    /// Get max selection index based on current view
    fn max_selection_index(&self) -> usize {
        match &self.today_summary {
            Some(summary) => match self.current_tab {
                Tab::Today => 0,
                Tab::Apps => summary.top_apps.len().saturating_sub(1),
                Tab::Categories => summary.category_breakdown.len().saturating_sub(1),
            },
            None => 0,
        }
    }
    
    /// Get today's total screen time formatted
    pub fn total_time_str(&self) -> String {
        match &self.today_summary {
            Some(s) => s.format_total_time(),
            None => "0h 0m".to_string(),
        }
    }
    
    /// Get progress through the day (0.0 - 1.0)
    /// Based on a target of 8 hours screen time
    pub fn day_progress(&self) -> f64 {
        const TARGET_SECS: f64 = 8.0 * 3600.0; // 8 hours
        match &self.today_summary {
            Some(s) => (s.total_screen_time_secs as f64 / TARGET_SECS).min(1.0),
            None => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_new() {
        let app = App::new(":memory:");
        assert!(!app.should_quit);
        assert_eq!(app.current_tab, Tab::Today);
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_tab_navigation() {
        let mut app = App::new(":memory:");
        
        assert_eq!(app.current_tab, Tab::Today);
        
        app.next_tab();
        assert_eq!(app.current_tab, Tab::Apps);
        
        app.next_tab();
        assert_eq!(app.current_tab, Tab::Categories);
        
        app.next_tab();
        assert_eq!(app.current_tab, Tab::Today);
        
        app.prev_tab();
        assert_eq!(app.current_tab, Tab::Categories);
    }

    #[test]
    fn test_quit() {
        let mut app = App::new(":memory:");
        assert!(!app.should_quit);
        app.quit();
        assert!(app.should_quit);
    }

    #[test]
    fn test_selection() {
        let mut app = App::new(":memory:");
        
        assert_eq!(app.selected_index, 0);
        
        app.select_prev();
        assert_eq!(app.selected_index, 0); // Can't go below 0
        
        // Without data, max is 0
        app.select_next();
        assert_eq!(app.selected_index, 0);
    }

    #[test]
    fn test_day_progress() {
        let app = App::new(":memory:");
        assert_eq!(app.day_progress(), 0.0);
    }

    #[test]
    fn test_total_time_str() {
        let app = App::new(":memory:");
        assert_eq!(app.total_time_str(), "0h 0m");
    }
}
