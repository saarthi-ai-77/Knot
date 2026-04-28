use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

/// A simple debouncer for file system events
pub struct EventDebouncer<T> {
    delay: Duration,
    pending: Arc<Mutex<HashMap<String, (T, Instant)>>>,
}

impl<T: Clone> EventDebouncer<T> {
    pub fn new(delay_ms: u64) -> Self {
        Self {
            delay: Duration::from_millis(delay_ms),
            pending: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Add an event to be debounced
    /// Returns true if this is a new event, false if it was merged with a pending event
    pub fn add_event(&self, key: String, event: T) -> bool {
        let mut pending = self.pending.lock().unwrap();
        let now = Instant::now();
        
        let is_new = !pending.contains_key(&key);
        pending.insert(key, (event, now));
        
        is_new
    }
    
    /// Get all events that have passed the debounce delay
    pub fn flush_due_events(&self) -> Vec<(String, T)> {
        let mut pending = self.pending.lock().unwrap();
        let now = Instant::now();
        let due: Vec<_> = pending
            .iter()
            .filter(|(_, (_, instant))| now.duration_since(*instant) >= self.delay)
            .map(|(k, (e, _))| (k.clone(), e.clone()))
            .collect();
        
        for (key, _) in &due {
            pending.remove(key);
        }
        
        due
    }
    
    /// Check if there are any pending events
    pub fn has_pending(&self) -> bool {
        let pending = self.pending.lock().unwrap();
        !pending.is_empty()
    }
    
    /// Clear all pending events
    pub fn clear(&self) {
        let mut pending = self.pending.lock().unwrap();
        pending.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    
    #[test]
    fn test_debouncer_basic() {
        let debouncer: EventDebouncer<String> = EventDebouncer::new(100);
        
        assert!(debouncer.add_event("file1".to_string(), "content1".to_string()));
        assert!(!debouncer.add_event("file1".to_string(), "content2".to_string())); // Same key, should merge
        
        assert!(debouncer.has_pending());
        
        sleep(Duration::from_millis(150));
        
        let events = debouncer.flush_due_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "file1");
        assert_eq!(events[0].1, "content2"); // Last content should win
        
        assert!(!debouncer.has_pending());
    }
    
    #[test]
    fn test_debouncer_multiple_keys() {
        let debouncer: EventDebouncer<String> = EventDebouncer::new(100);
        
        debouncer.add_event("file1".to_string(), "a".to_string());
        debouncer.add_event("file2".to_string(), "b".to_string());
        
        sleep(Duration::from_millis(150));
        
        let events = debouncer.flush_due_events();
        assert_eq!(events.len(), 2);
    }
}
