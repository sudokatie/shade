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
    }

    impl Default for ShadeConfig {
        fn default() -> Self {
            Self {
                db_path: default_db_path(),
                idle_timeout_secs: default_idle_timeout(),
                collection_interval_secs: default_collection_interval(),
                track_window_titles: false,
                categories: Vec::new(),
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
}
