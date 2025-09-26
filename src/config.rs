//! Configuration management for Terminalist
//!
//! This module handles loading, parsing, and validation of configuration files.

use crate::constants::{CONFIG_GENERATED, SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH};
use crate::utils::datetime;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub ui: UiConfig,
    pub sync: SyncConfig,
    pub display: DisplayConfig,
    pub logging: LoggingConfig,
    pub backends: BackendsConfig,
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Default project to open on startup
    /// Options: "inbox", "today", "tomorrow", "upcoming", project ID, or project name
    pub default_project: String,
    /// Enable mouse support
    pub mouse_enabled: bool,
    /// Sidebar width in columns
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

/// Backend configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BackendsConfig {
    /// Default backend to use for new items
    pub default_backend: String,
    /// Individual backend configurations
    pub todoist: TodoistBackendConfig,
    // Future backends can be added here:
    // pub local: LocalBackendConfig,
    // pub caldav: CalDAVBackendConfig,
}

/// Todoist backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TodoistBackendConfig {
    /// Enable this backend
    pub enabled: bool,
    /// Environment variable containing the API token
    pub api_token_env: String,
    /// Human-readable name for this backend instance
    pub name: String,
    /// Custom backend ID (optional, defaults to "todoist")
    pub backend_id: Option<String>,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            default_project: "today".to_string(),
            mouse_enabled: true,
            sidebar_width: SIDEBAR_DEFAULT_WIDTH,
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

impl Default for BackendsConfig {
    fn default() -> Self {
        Self {
            default_backend: "todoist".to_string(),
            todoist: TodoistBackendConfig::default(),
        }
    }
}

impl Default for TodoistBackendConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_token_env: "TODOIST_API_TOKEN".to_string(),
            name: "Todoist".to_string(),
            backend_id: None, // Defaults to "todoist"
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
        if self.ui.sidebar_width < SIDEBAR_MIN_WIDTH || self.ui.sidebar_width > SIDEBAR_MAX_WIDTH {
            anyhow::bail!(
                "sidebar_width must be between {} and {} columns, got {}",
                SIDEBAR_MIN_WIDTH,
                SIDEBAR_MAX_WIDTH,
                self.ui.sidebar_width
            );
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

        // Validate backend configuration
        self.validate_backends()?;

        Ok(())
    }

    /// Validate backend configurations
    fn validate_backends(&self) -> Result<()> {
        // Check if default backend is valid
        let available_backends = self.get_available_backend_ids();
        if !available_backends.contains(&self.backends.default_backend) {
            anyhow::bail!(
                "default_backend '{}' is not available. Available backends: {}",
                self.backends.default_backend,
                available_backends.join(", ")
            );
        }

        // Validate Todoist backend config
        if self.backends.todoist.enabled {
            if self.backends.todoist.api_token_env.is_empty() {
                anyhow::bail!("todoist.api_token_env cannot be empty when todoist backend is enabled");
            }
            if self.backends.todoist.name.is_empty() {
                anyhow::bail!("todoist.name cannot be empty when todoist backend is enabled");
            }
        }

        Ok(())
    }

    /// Get list of available backend IDs based on configuration
    pub fn get_available_backend_ids(&self) -> Vec<String> {
        let mut backends = Vec::new();

        if self.backends.todoist.enabled {
            let backend_id = self.backends.todoist.backend_id
                .as_deref()
                .unwrap_or("todoist")
                .to_string();
            backends.push(backend_id);
        }

        backends
    }

    /// Check if a specific backend is enabled
    pub fn is_backend_enabled(&self, backend_type: &str) -> bool {
        match backend_type {
            "todoist" => self.backends.todoist.enabled,
            _ => false,
        }
    }

    /// Get the Todoist backend configuration
    pub fn get_todoist_config(&self) -> &TodoistBackendConfig {
        &self.backends.todoist
    }

    /// Get the environment variable name for a backend's API token
    pub fn get_api_token_env_var(&self, backend_type: &str) -> Option<&str> {
        match backend_type {
            "todoist" => {
                if self.backends.todoist.enabled {
                    Some(&self.backends.todoist.api_token_env)
                } else {
                    None
                }
            }
            _ => None,
        }
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

        println!("{}: {}", CONFIG_GENERATED, path.as_ref().display());
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
