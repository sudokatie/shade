//! Collector trait and background thread utilities

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Trait for data collectors
/// 
/// Collectors gather data from various sources and store it in the database.
/// They run in background threads and can be started/stopped.
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

/// Handle for a background collector thread
pub struct CollectorHandle {
    /// Name of the collector
    name: String,
    /// Flag to signal the thread to stop
    running: Arc<AtomicBool>,
    /// Thread handle
    handle: Option<JoinHandle<()>>,
}

impl CollectorHandle {
    /// Create a new collector handle
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
        }
    }
    
    /// Get the name of this collector
    pub fn name(&self) -> &str {
        &self.name
    }
    
    /// Check if the collector is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
    
    /// Get a clone of the running flag for use in the collector thread
    pub fn running_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.running)
    }
    
    /// Start the collector with the given work function
    /// 
    /// The work function receives a reference to the running flag and should
    /// check it periodically to know when to stop.
    pub fn start<F>(&mut self, work: F) -> anyhow::Result<()>
    where
        F: FnOnce(Arc<AtomicBool>) + Send + 'static,
    {
        if self.is_running() {
            anyhow::bail!("Collector '{}' is already running", self.name);
        }
        
        self.running.store(true, Ordering::SeqCst);
        let running = Arc::clone(&self.running);
        
        let handle = thread::spawn(move || {
            work(running);
        });
        
        self.handle = Some(handle);
        Ok(())
    }
    
    /// Stop the collector and wait for the thread to finish
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        
        if let Some(handle) = self.handle.take() {
            // Give the thread a chance to finish gracefully
            let _ = handle.join();
        }
    }
}

impl Drop for CollectorHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    #[test]
    fn test_collector_handle_new() {
        let handle = CollectorHandle::new("test");
        assert_eq!(handle.name(), "test");
        assert!(!handle.is_running());
    }

    #[test]
    fn test_collector_handle_start_stop() {
        let mut handle = CollectorHandle::new("test");
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);
        
        handle.start(move |running| {
            while running.load(Ordering::SeqCst) {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(10));
            }
        }).unwrap();
        
        assert!(handle.is_running());
        
        // Let it run a bit
        thread::sleep(Duration::from_millis(50));
        
        handle.stop();
        
        assert!(!handle.is_running());
        assert!(counter.load(Ordering::SeqCst) > 0);
    }

    #[test]
    fn test_collector_handle_double_start_fails() {
        let mut handle = CollectorHandle::new("test");
        
        handle.start(|running| {
            while running.load(Ordering::SeqCst) {
                thread::sleep(Duration::from_millis(10));
            }
        }).unwrap();
        
        let result = handle.start(|_| {});
        assert!(result.is_err());
        
        handle.stop();
    }

    #[test]
    fn test_collector_handle_drop_stops_thread() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);
        
        {
            let mut handle = CollectorHandle::new("test");
            handle.start(move |running| {
                while running.load(Ordering::SeqCst) {
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(10));
                }
            }).unwrap();
            
            thread::sleep(Duration::from_millis(30));
            // handle dropped here
        }
        
        let count_at_drop = counter.load(Ordering::SeqCst);
        thread::sleep(Duration::from_millis(50));
        let count_after = counter.load(Ordering::SeqCst);
        
        // Counter should have stopped incrementing after drop
        assert_eq!(count_at_drop, count_after);
    }

    /// Mock collector for testing
    struct MockCollector {
        handle: CollectorHandle,
        tick_count: Arc<AtomicUsize>,
    }

    impl MockCollector {
        fn new() -> Self {
            Self {
                handle: CollectorHandle::new("mock"),
                tick_count: Arc::new(AtomicUsize::new(0)),
            }
        }
        
        fn tick_count(&self) -> usize {
            self.tick_count.load(Ordering::SeqCst)
        }
    }

    impl Collector for MockCollector {
        fn name(&self) -> &str {
            self.handle.name()
        }
        
        fn start(&mut self) -> anyhow::Result<()> {
            let tick_count = Arc::clone(&self.tick_count);
            self.handle.start(move |running| {
                while running.load(Ordering::SeqCst) {
                    tick_count.fetch_add(1, Ordering::SeqCst);
                    thread::sleep(Duration::from_millis(10));
                }
            })
        }
        
        fn stop(&mut self) {
            self.handle.stop();
        }
        
        fn is_running(&self) -> bool {
            self.handle.is_running()
        }
    }

    #[test]
    fn test_mock_collector_trait_impl() {
        let mut collector = MockCollector::new();
        
        assert_eq!(collector.name(), "mock");
        assert!(!collector.is_running());
        assert_eq!(collector.tick_count(), 0);
        
        collector.start().unwrap();
        assert!(collector.is_running());
        
        thread::sleep(Duration::from_millis(50));
        
        collector.stop();
        assert!(!collector.is_running());
        assert!(collector.tick_count() > 0);
    }
}
