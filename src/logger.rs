use crate::constants::MEMORY_LOGS_LIMIT;
use chrono::Utc;
use log::{Log, Metadata, Record};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

/// Global in-memory log storage for UI display
static MEMORY_LOGS: LazyLock<Mutex<VecDeque<String>>> =
    LazyLock::new(|| Mutex::new(VecDeque::with_capacity(MEMORY_LOGS_LIMIT)));

static LOGGER: TerminalistLogger = TerminalistLogger { file: Mutex::new(None) };

/// Logger writing to memory (for the UI) and, when enabled, to a file.
struct TerminalistLogger {
    file: Mutex<Option<File>>,
}

/// Initialize the logger, optionally also writing to the log file.
pub fn init_logger(enabled: bool) -> io::Result<()> {
    if enabled {
        let log_file_path = get_log_file_path()?;

        if let Some(parent) = log_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let log_file = OpenOptions::new().create(true).append(true).open(&log_file_path)?;
        *LOGGER.file.lock().map_err(|_| io::Error::other("logger lock poisoned"))? = Some(log_file);
    }

    log::set_logger(&LOGGER).map_err(|e| io::Error::other(e.to_string()))?;
    log::set_max_level(log::LevelFilter::Info);
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

impl Log for TerminalistLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let message = format!("{}", record.args());

        if let Ok(mut logs) = MEMORY_LOGS.lock() {
            logs.push_back(message.clone());
            // Keep only the last MEMORY_LOGS_LIMIT entries
            while logs.len() > MEMORY_LOGS_LIMIT {
                logs.pop_front();
            }
        }

        if let Ok(mut file) = self.file.lock() {
            if let Some(file) = file.as_mut() {
                let _ = writeln!(file, "[{}] {}", Utc::now().format("%H:%M:%S%.3f"), message);
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.file.lock() {
            if let Some(file) = file.as_mut() {
                let _ = file.flush();
            }
        }
    }
}
