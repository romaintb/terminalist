use crate::constants::MEMORY_LOGS_LIMIT;
use chrono::Utc;
use log::Record;
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Global in-memory log storage for UI display
static MEMORY_LOGS: once_cell::sync::Lazy<Arc<Mutex<VecDeque<String>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(VecDeque::with_capacity(5000))));

/// Initialize the fern logger with file and memory outputs
pub fn init_logger(enabled: bool) -> io::Result<()> {
    eprintln!("LOGGER DEBUG: init_logger called with enabled={}", enabled);
    if !enabled {
        // Set up a logger that only writes to memory
        // Use Trace level so MemoryLogger receives all logs
        fern::Dispatch::new()
            .level(log::LevelFilter::Info)
            .chain(Box::new(MemoryLogger) as Box<dyn log::Log>)
            .apply()
            .map_err(io::Error::other)?;
        return Ok(());
    }

    let log_file_path = get_log_file_path()?;

    // Write a test message to stderr to confirm this code runs
    eprintln!("LOGGER DEBUG: Attempting to create log at: {}", log_file_path.display());

    // Ensure the config directory exists
    if let Some(parent) = log_file_path.parent() {
        std::fs::create_dir_all(parent)?;
        eprintln!("LOGGER DEBUG: Created directory: {}", parent.display());
    }

    // Test file creation manually first
    {
        let mut test_file = OpenOptions::new().create(true).append(true).open(&log_file_path)?;
        use std::io::Write;
        writeln!(test_file, "[{}] Logger initialization test", Utc::now().format("%H:%M:%S%.3f"))?;
        eprintln!("LOGGER DEBUG: Successfully wrote test line to log file");
    }

    // Configure fern logger with file output
    fern::Dispatch::new()
        .format(|out, message, _record| out.finish(format_args!("[{}] {}", Utc::now().format("%H:%M:%S%.3f"), message)))
        .level(log::LevelFilter::Info)
        .chain(fern::Output::file(OpenOptions::new().create(true).append(true).open(&log_file_path)?, "\n"))
        .chain(Box::new(MemoryLogger) as Box<dyn log::Log>)
        .apply()
        .map_err(io::Error::other)?;

    eprintln!("LOGGER DEBUG: Fern logger configured successfully");

    Ok(())
}

/// Get the standard log file path
pub fn get_log_file_path() -> io::Result<PathBuf> {
    let config_dir =
        dirs::config_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Config directory not found"))?;

    Ok(config_dir.join("terminalist").join("terminalist.log"))
}

/// Get all logs from memory (for UI display)
pub fn get_memory_logs() -> Vec<String> {
    if let Ok(logs) = MEMORY_LOGS.lock() {
        logs.iter().rev().cloned().collect()
    } else {
        Vec::new()
    }
}

/// Clear all logs from memory
pub fn clear_memory_logs() {
    if let Ok(mut logs) = MEMORY_LOGS.lock() {
        logs.clear();
    }
}

/// Custom logger that stores logs in memory for UI display
struct MemoryLogger;

impl log::Log for MemoryLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let formatted = format!("{}", record.args());

            if let Ok(mut logs) = MEMORY_LOGS.lock() {
                logs.push_back(formatted);
                // Keep only last 5000 entries
                while logs.len() > MEMORY_LOGS_LIMIT {
                    logs.pop_front();
                }
            }
        }
    }

    fn flush(&self) {}
}
