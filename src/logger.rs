use chrono::Utc;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Shared logger that can be used across the application
#[derive(Clone)]
pub struct Logger {
    logs: Arc<Mutex<Vec<String>>>,
    log_file: Option<PathBuf>,
    enabled: bool,
}

impl Logger {
    /// Create a new logger with file logging enabled
    pub fn new_with_file_logging() -> io::Result<Self> {
        let log_file_path = Self::get_log_file_path()?;
        
        // Ensure the config directory exists
        if let Some(parent) = log_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        Ok(Self {
            logs: Arc::new(Mutex::new(Vec::new())),
            log_file: Some(log_file_path),
            enabled: true,
        })
    }
    
    /// Create a new logger without file logging (in-memory only)
    pub fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
            log_file: None,
            enabled: false,
        }
    }
    
    /// Get the standard log file path
    fn get_log_file_path() -> io::Result<PathBuf> {
        let home_dir = dirs::home_dir()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Home directory not found"))?;
        
        Ok(home_dir.join(".config").join("terminalist").join("terminalist.log"))
    }

    /// Add a log entry
    pub fn log(&self, message: String) {
        let timestamp = Utc::now().format("%H:%M:%S%.3f").to_string();
        let formatted_message = format!("[{}] {}", timestamp, message);

        // Always store in memory for UI display
        if let Ok(mut logs) = self.logs.lock() {
            logs.push(formatted_message.clone());
        }
        
        // Write to file if file logging is enabled
        if self.enabled {
            if let Some(ref log_file_path) = self.log_file {
                if let Ok(mut file) = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(log_file_path)
                {
                    let _ = writeln!(file, "{}", formatted_message);
                    let _ = file.flush();
                }
            }
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
