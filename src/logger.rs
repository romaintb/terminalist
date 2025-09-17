use chrono::Utc;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Shared logger that can be used across the application
#[derive(Clone)]
pub struct Logger {
    logs: Arc<Mutex<Vec<String>>>,
    file_writer: Option<Arc<Mutex<BufWriter<File>>>>,
    enabled: bool,
}

impl Logger {
    /// Create a new logger based on configuration
    pub fn from_config(logging_enabled: bool) -> io::Result<Self> {
        if logging_enabled {
            Self::new_with_file_logging()
        } else {
            Ok(Self::new())
        }
    }

    /// Create a new logger with file logging enabled
    pub fn new_with_file_logging() -> io::Result<Self> {
        let log_file_path = Self::get_log_file_path()?;

        // Ensure the config directory exists
        if let Some(parent) = log_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open the file once and create a buffered writer
        let file = OpenOptions::new().create(true).append(true).open(&log_file_path)?;
        let buf_writer = BufWriter::new(file);
        let file_writer = Some(Arc::new(Mutex::new(buf_writer)));

        Ok(Self {
            logs: Arc::new(Mutex::new(Vec::new())),
            file_writer,
            enabled: true,
        })
    }

    /// Create a new logger without file logging (in-memory only)
    pub fn new() -> Self {
        Self {
            logs: Arc::new(Mutex::new(Vec::new())),
            file_writer: None,
            enabled: false,
        }
    }

    /// Get the standard log file path
    pub fn get_log_file_path() -> io::Result<PathBuf> {
        let config_dir =
            dirs::config_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Config directory not found"))?;

        Ok(config_dir.join("terminalist").join("terminalist.log"))
    }

    /// Add a log entry
    pub fn log(&self, message: String) {
        let timestamp = Utc::now().format("%H:%M:%S%.3f").to_string();
        let formatted_message = format!("[{}] {}", timestamp, message);

        // Always store in memory for UI display
        if let Ok(mut logs) = self.logs.lock() {
            const MAX_LOGS: usize = 5000;
            logs.push(formatted_message.clone());
            if logs.len() > MAX_LOGS {
                let drop_n = logs.len() - MAX_LOGS;
                logs.drain(0..drop_n);
            }
        }

        // Write to file if file logging is enabled
        if self.enabled {
            if let Some(ref file_writer) = self.file_writer {
                if let Ok(mut writer) = file_writer.lock() {
                    // Write to buffered writer, ignore errors to avoid logging recursion
                    let _ = writeln!(writer, "{}", formatted_message);
                    // Let BufWriter handle flushing automatically, don't flush on every entry
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

    /// Check if logging is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Check if file writer exists
    pub fn has_file_writer(&self) -> bool {
        self.file_writer.is_some()
    }

    /// Get a reference to the file writer for testing
    pub fn file_writer(&self) -> &Option<Arc<Mutex<BufWriter<File>>>> {
        &self.file_writer
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

