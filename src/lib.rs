//! Shade - Privacy-first personal analytics
//!
//! All data stays local. No cloud. No accounts.

pub mod analytics;
pub mod cli;
pub mod collectors;
pub mod db;
pub mod export;
pub mod tui;

/// Application configuration
pub mod config {
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;

    /// Main configuration for Shade
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ShadeConfig {
        /// Database path
        #[serde(default = "default_db_path")]
        pub db_path: PathBuf,

        /// Idle timeout in seconds
        #[serde(default = "default_idle_timeout")]
        pub idle_timeout_secs: u64,

        /// Collection interval in seconds
        #[serde(default = "default_collection_interval")]
        pub collection_interval_secs: u64,

        /// Whether to track window titles (privacy-sensitive)
        #[serde(default)]
        pub track_window_titles: bool,

        /// App categories for classification
        #[serde(default)]
        pub categories: Vec<CategoryConfig>,

        /// Time goals for apps or categories
        #[serde(default)]
        pub goals: Vec<TimeGoal>,
    }

    impl Default for ShadeConfig {
        fn default() -> Self {
            Self {
                db_path: default_db_path(),
                idle_timeout_secs: default_idle_timeout(),
                collection_interval_secs: default_collection_interval(),
                track_window_titles: false,
                categories: Vec::new(),
                goals: Vec::new(),
            }
        }
    }

    impl ShadeConfig {
        /// Load config from file or return default
        pub fn load() -> anyhow::Result<Self> {
            let config_path = config_path();
            if config_path.exists() {
                let content = std::fs::read_to_string(&config_path)?;
                let config: ShadeConfig = serde_yaml::from_str(&content)?;
                Ok(config)
            } else {
                Ok(Self::default())
            }
        }

        /// Save config to file
        pub fn save(&self) -> anyhow::Result<()> {
            let config_path = config_path();
            if let Some(parent) = config_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let content = serde_yaml::to_string(self)?;
            std::fs::write(&config_path, content)?;
            Ok(())
        }
    }

    /// Category configuration
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CategoryConfig {
        pub name: String,
        /// Bundle IDs or app names that belong to this category
        pub patterns: Vec<String>,
    }

    /// Time goal for limiting screen time
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct TimeGoal {
        /// Target - can be an app bundle ID or category name
        pub target: String,
        /// Whether target is a category (true) or app (false)
        #[serde(default)]
        pub is_category: bool,
        /// Daily limit in minutes
        pub daily_limit_minutes: u32,
        /// Warn at this percentage (default 80)
        #[serde(default = "default_warn_percent")]
        pub warn_at_percent: u8,
    }

    fn default_warn_percent() -> u8 {
        80
    }

    impl TimeGoal {
        /// Create a new goal for an app
        pub fn for_app(bundle_id: &str, daily_limit_minutes: u32) -> Self {
            Self {
                target: bundle_id.to_string(),
                is_category: false,
                daily_limit_minutes,
                warn_at_percent: 80,
            }
        }

        /// Create a new goal for a category
        pub fn for_category(category: &str, daily_limit_minutes: u32) -> Self {
            Self {
                target: category.to_string(),
                is_category: true,
                daily_limit_minutes,
                warn_at_percent: 80,
            }
        }
    }

    impl ShadeConfig {
        /// Get user-defined categories as a HashMap
        pub fn category_map(&self) -> std::collections::HashMap<String, String> {
            let mut map = std::collections::HashMap::new();
            for cat in &self.categories {
                for pattern in &cat.patterns {
                    map.insert(pattern.clone(), cat.name.clone());
                }
            }
            map
        }

        /// Add an app to a category
        pub fn add_to_category(&mut self, bundle_id: &str, category: &str) {
            // Find existing category or create new one
            if let Some(cat) = self.categories.iter_mut().find(|c| c.name == category) {
                if !cat.patterns.contains(&bundle_id.to_string()) {
                    cat.patterns.push(bundle_id.to_string());
                }
            } else {
                self.categories.push(CategoryConfig {
                    name: category.to_string(),
                    patterns: vec![bundle_id.to_string()],
                });
            }
        }

        /// Remove an app from a category
        pub fn remove_from_category(&mut self, bundle_id: &str, category: &str) -> bool {
            if let Some(cat) = self.categories.iter_mut().find(|c| c.name == category) {
                let original_len = cat.patterns.len();
                cat.patterns.retain(|p| p != bundle_id);
                cat.patterns.len() < original_len
            } else {
                false
            }
        }

        /// List all user-defined categories
        pub fn list_categories(&self) -> Vec<(&str, usize)> {
            self.categories
                .iter()
                .map(|c| (c.name.as_str(), c.patterns.len()))
                .collect()
        }

        /// Add a time goal
        pub fn add_goal(&mut self, goal: TimeGoal) -> bool {
            // Check for duplicate target
            if self.goals.iter().any(|g| g.target == goal.target && g.is_category == goal.is_category) {
                return false;
            }
            self.goals.push(goal);
            true
        }

        /// Remove a time goal by target
        pub fn remove_goal(&mut self, target: &str, is_category: bool) -> bool {
            let original_len = self.goals.len();
            self.goals.retain(|g| !(g.target == target && g.is_category == is_category));
            self.goals.len() < original_len
        }

        /// Get a goal by target
        pub fn get_goal(&self, target: &str, is_category: bool) -> Option<&TimeGoal> {
            self.goals.iter().find(|g| g.target == target && g.is_category == is_category)
        }

        /// List all goals
        pub fn list_goals(&self) -> &[TimeGoal] {
            &self.goals
        }

        /// Update goal limit
        pub fn update_goal_limit(&mut self, target: &str, is_category: bool, new_limit: u32) -> bool {
            if let Some(goal) = self.goals.iter_mut().find(|g| g.target == target && g.is_category == is_category) {
                goal.daily_limit_minutes = new_limit;
                true
            } else {
                false
            }
        }
    }

    fn default_db_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".shade")
            .join("shade.db")
    }

    fn default_idle_timeout() -> u64 {
        300 // 5 minutes
    }

    fn default_collection_interval() -> u64 {
        1 // 1 second
    }

    fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".shade")
            .join("config.yaml")
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_category_map_empty() {
            let config = ShadeConfig::default();
            assert!(config.category_map().is_empty());
        }

        #[test]
        fn test_category_map_with_categories() {
            let mut config = ShadeConfig::default();
            config.categories.push(CategoryConfig {
                name: "Work".to_string(),
                patterns: vec!["com.slack.Slack".to_string(), "com.microsoft.teams".to_string()],
            });

            let map = config.category_map();
            assert_eq!(map.get("com.slack.Slack"), Some(&"Work".to_string()));
            assert_eq!(map.get("com.microsoft.teams"), Some(&"Work".to_string()));
        }

        #[test]
        fn test_add_to_category_new() {
            let mut config = ShadeConfig::default();
            config.add_to_category("com.example.app", "Custom");

            assert_eq!(config.categories.len(), 1);
            assert_eq!(config.categories[0].name, "Custom");
            assert!(config.categories[0].patterns.contains(&"com.example.app".to_string()));
        }

        #[test]
        fn test_add_to_category_existing() {
            let mut config = ShadeConfig::default();
            config.add_to_category("com.example.app1", "Custom");
            config.add_to_category("com.example.app2", "Custom");

            assert_eq!(config.categories.len(), 1);
            assert_eq!(config.categories[0].patterns.len(), 2);
        }

        #[test]
        fn test_add_to_category_no_duplicate() {
            let mut config = ShadeConfig::default();
            config.add_to_category("com.example.app", "Custom");
            config.add_to_category("com.example.app", "Custom");

            assert_eq!(config.categories[0].patterns.len(), 1);
        }

        #[test]
        fn test_remove_from_category() {
            let mut config = ShadeConfig::default();
            config.add_to_category("com.example.app", "Custom");
            
            let removed = config.remove_from_category("com.example.app", "Custom");
            assert!(removed);
            assert!(config.categories[0].patterns.is_empty());
        }

        #[test]
        fn test_remove_from_category_not_found() {
            let mut config = ShadeConfig::default();
            config.add_to_category("com.example.app", "Custom");
            
            let removed = config.remove_from_category("com.other.app", "Custom");
            assert!(!removed);
        }

        #[test]
        fn test_list_categories() {
            let mut config = ShadeConfig::default();
            config.add_to_category("com.app1", "Work");
            config.add_to_category("com.app2", "Work");
            config.add_to_category("com.app3", "Fun");

            let list = config.list_categories();
            assert_eq!(list.len(), 2);
            assert!(list.iter().any(|(name, count)| *name == "Work" && *count == 2));
            assert!(list.iter().any(|(name, count)| *name == "Fun" && *count == 1));
        }

        #[test]
        fn test_add_goal_app() {
            let mut config = ShadeConfig::default();
            let goal = TimeGoal::for_app("com.example.app", 120);
            assert!(config.add_goal(goal));
            assert_eq!(config.goals.len(), 1);
            assert_eq!(config.goals[0].target, "com.example.app");
            assert!(!config.goals[0].is_category);
        }

        #[test]
        fn test_add_goal_category() {
            let mut config = ShadeConfig::default();
            let goal = TimeGoal::for_category("Social", 60);
            assert!(config.add_goal(goal));
            assert_eq!(config.goals.len(), 1);
            assert!(config.goals[0].is_category);
        }

        #[test]
        fn test_add_goal_no_duplicate() {
            let mut config = ShadeConfig::default();
            config.add_goal(TimeGoal::for_app("com.example.app", 120));
            // Same target, should fail
            assert!(!config.add_goal(TimeGoal::for_app("com.example.app", 60)));
            assert_eq!(config.goals.len(), 1);
        }

        #[test]
        fn test_remove_goal() {
            let mut config = ShadeConfig::default();
            config.add_goal(TimeGoal::for_app("com.example.app", 120));
            assert!(config.remove_goal("com.example.app", false));
            assert!(config.goals.is_empty());
        }

        #[test]
        fn test_remove_goal_not_found() {
            let mut config = ShadeConfig::default();
            config.add_goal(TimeGoal::for_app("com.example.app", 120));
            assert!(!config.remove_goal("com.other.app", false));
        }

        #[test]
        fn test_get_goal() {
            let mut config = ShadeConfig::default();
            config.add_goal(TimeGoal::for_app("com.example.app", 120));
            
            let goal = config.get_goal("com.example.app", false);
            assert!(goal.is_some());
            assert_eq!(goal.unwrap().daily_limit_minutes, 120);
        }

        #[test]
        fn test_update_goal_limit() {
            let mut config = ShadeConfig::default();
            config.add_goal(TimeGoal::for_app("com.example.app", 120));
            
            assert!(config.update_goal_limit("com.example.app", false, 90));
            assert_eq!(config.goals[0].daily_limit_minutes, 90);
        }

        #[test]
        fn test_time_goal_warn_percent_default() {
            let goal = TimeGoal::for_app("com.example.app", 120);
            assert_eq!(goal.warn_at_percent, 80);
        }
    }
}
