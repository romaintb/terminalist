//! Configuration management for Terminalist
//!
//! This module handles loading, parsing, and validation of configuration files.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::utils::datetime;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub ui: UiConfig,
    pub sync: SyncConfig,
    pub display: DisplayConfig,
    pub logging: LoggingConfig,
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Default project to open on startup
    /// Options: "inbox", "today", "tomorrow", "upcoming", or project ID
    pub default_project: String,
    /// Enable mouse support
    pub mouse_enabled: bool,
    /// Sidebar width percentage (10-40)
    pub sidebar_width: u16,
}

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SyncConfig {
    /// Auto-sync interval in minutes (0 = disabled, manual sync only)
    pub auto_sync_interval_minutes: u64,
}

/// Display configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    /// Date format for task due dates
    pub date_format: String,
    /// Time format for datetime fields
    pub time_format: String,
    /// Show task descriptions in list view
    pub show_descriptions: bool,
    /// Show task durations
    pub show_durations: bool,
    /// Show task labels
    pub show_labels: bool,
    /// Show project colors
    pub show_project_colors: bool,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LoggingConfig {
    /// Enable logging
    pub enabled: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            default_project: "today".to_string(),
            mouse_enabled: true,
            sidebar_width: 25,
        }
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            auto_sync_interval_minutes: 5,
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            date_format: datetime::TODOIST_DATE_FORMAT.to_string(),
            time_format: "%H:%M".to_string(),
            show_descriptions: true,
            show_durations: true,
            show_labels: true,
            show_project_colors: false,
        }
    }
}

impl Config {
    /// Load configuration from file or return defaults
    pub fn load() -> Result<Self> {
        let config_path = Self::find_config_file()?;

        if let Some(path) = config_path {
            Self::load_from_file(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load configuration from a specific file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;

        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.as_ref().display()))?;

        config.validate()?;
        Ok(config)
    }

    /// Find configuration file in order of precedence
    fn find_config_file() -> Result<Option<PathBuf>> {
        // 1. Check current directory
        let current_dir_config = PathBuf::from("terminalist.toml");
        if current_dir_config.exists() {
            return Ok(Some(current_dir_config));
        }

        // 2. Check XDG config directory
        if let Some(config_dir) = dirs::config_dir() {
            let xdg_config = config_dir.join("terminalist").join("config.toml");
            if xdg_config.exists() {
                return Ok(Some(xdg_config));
            }
        }

        Ok(None)
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate UI settings
        if self.ui.sidebar_width < 10 || self.ui.sidebar_width > 40 {
            anyhow::bail!("sidebar_width must be between 10 and 40, got {}", self.ui.sidebar_width);
        }

        // Validate default project
        let valid_projects = ["inbox", "today", "tomorrow", "upcoming"];
        if !valid_projects.contains(&self.ui.default_project.as_str()) {
            // If it's not a predefined value, assume it's a project ID
            // We could add more validation here if needed
        }

        // Validate sync interval
        if self.sync.auto_sync_interval_minutes > 1440 {
            anyhow::bail!("auto_sync_interval_minutes cannot exceed 1440 (24 hours)");
        }

        // Validate date/time formats
        if let Err(e) = chrono::NaiveDate::parse_from_str("2025-01-01", &self.display.date_format) {
            anyhow::bail!("Invalid date_format '{}': {}", self.display.date_format, e);
        }

        if let Err(e) = chrono::NaiveTime::parse_from_str("12:00", &self.display.time_format) {
            anyhow::bail!("Invalid time_format '{}': {}", self.display.time_format, e);
        }

        Ok(())
    }

    /// Generate default configuration file
    pub fn generate_default_config<P: AsRef<Path>>(path: P) -> Result<()> {
        let config = Self::default();
        let toml_content = toml::to_string_pretty(&config).context("Failed to serialize default config")?;

        // Add header comment
        let header = format!(
            "# Terminalist Configuration File\n# Generated on {}\n\n",
            chrono::Local::now().format(datetime::TODOIST_DATE_FORMAT)
        );

        let full_content = header + &toml_content;

        // Ensure the parent directory exists
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
        }

        std::fs::write(&path, full_content)
            .with_context(|| format!("Failed to write config file: {}", path.as_ref().display()))?;

        println!("✅ Generated default configuration file: {}", path.as_ref().display());
        Ok(())
    }

    /// Get the XDG config directory path
    pub fn get_xdg_config_dir() -> Result<PathBuf> {
        dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))
            .map(|dir| dir.join("terminalist"))
    }

    /// Get the default config file path
    pub fn get_default_config_path() -> Result<PathBuf> {
        Ok(Self::get_xdg_config_dir()?.join("config.toml"))
    }
}
