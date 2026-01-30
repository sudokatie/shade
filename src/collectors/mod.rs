//! Data collectors for Shade
//!
//! Collectors gather data from various sources (screen time, apps, etc.)

/// Trait for data collectors
pub trait Collector: Send {
    /// Get the name of this collector
    fn name(&self) -> &str;
    
    /// Start collecting data
    fn start(&mut self) -> anyhow::Result<()>;
    
    /// Stop collecting data
    fn stop(&mut self);
    
    /// Check if the collector is running
    fn is_running(&self) -> bool;
}
