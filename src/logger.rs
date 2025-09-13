use chrono::Utc;
use std::sync::{Arc, Mutex};

/// Shared logger that can be used across the application
#[derive(Clone)]
pub struct Logger {
    logs: Arc<Mutex<Vec<String>>>,
}

impl Logger {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a log entry
    pub fn log(&self, message: String) {
        let timestamp = Utc::now().format("%H:%M:%S%.3f").to_string();
        let formatted_message = format!("[{}] {}", timestamp, message);

        if let Ok(mut logs) = self.logs.lock() {
            logs.push(formatted_message);
        }
    }

    /// Get all logs sorted by date (newest first)
    pub fn get_logs(&self) -> Vec<String> {
        if let Ok(logs) = self.logs.lock() {
            let mut sorted_logs = logs.clone();
            // Reverse to show newest logs first (descending order by timestamp)
            sorted_logs.reverse();
            sorted_logs
        } else {
            Vec::new()
        }
    }

    /// Clear all logs
    pub fn clear(&self) {
        if let Ok(mut logs) = self.logs.lock() {
            logs.clear();
        }
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}
