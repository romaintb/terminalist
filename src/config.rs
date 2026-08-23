//! Configuration management for Terminalist
//!
//! This module handles loading, parsing, and validation of configuration files.

use crate::constants::{CONFIG_GENERATED, SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH};
use crate::theme::{RawTheme, Theme, ThemeWarning};
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
    #[serde(skip_serializing_if = "StorageConfig::is_unset")]
    pub storage: StorageConfig,
    pub theme: Theme,
}

/// Mirrors [`Config`], but with `theme` left unvalidated so a malformed color value
/// doesn't fail the whole config load. Used only by [`Config::load_from_file`].
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct RawConfig {
    ui: UiConfig,
    sync: SyncConfig,
    display: DisplayConfig,
    logging: LoggingConfig,
    storage: StorageConfig,
    theme: RawTheme,
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
    /// Show sidebar on startup
    pub sidebar_visible: bool,
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

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct StorageConfig {
    /// Directory holding the local SQLite cache.
    ///
    /// Unset means the platform data directory (`~/.local/share/terminalist` on Linux). A
    /// leading `~` is expanded; relative paths resolve against the working directory.
    pub data_dir: Option<PathBuf>,
}

impl StorageConfig {
    /// True when no override is configured, so the section is omitted from generated files.
    fn is_unset(&self) -> bool {
        self.data_dir.is_none()
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            default_project: "today".to_string(),
            mouse_enabled: true,
            sidebar_width: SIDEBAR_DEFAULT_WIDTH,
            sidebar_visible: true,
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
    /// Load configuration from file or return defaults.
    ///
    /// Returns any [`ThemeWarning`]s produced by falling back to default colors for
    /// individual `[theme]` values that were missing, malformed, or the wrong type.
    pub fn load() -> Result<(Self, Vec<ThemeWarning>)> {
        let config_path = Self::find_config_file()?;

        if let Some(path) = config_path {
            Self::load_from_file(&path)
        } else {
            Ok((Self::default(), Vec::new()))
        }
    }

    /// Load configuration from a specific file.
    ///
    /// The `[ui]`, `[sync]`, `[display]`, and `[logging]` sections are parsed strictly, same
    /// as before — a malformed value there still fails the whole load. The `[theme]` section
    /// is parsed leniently: any color that's missing, malformed, or the wrong type falls
    /// back to its built-in default rather than failing the load, and is reported as a
    /// [`ThemeWarning`] instead.
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<(Self, Vec<ThemeWarning>)> {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;

        let raw: RawConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.as_ref().display()))?;

        let (theme, warnings) = Theme::from_raw(raw.theme, &content);

        let config = Config {
            ui: raw.ui,
            sync: raw.sync,
            display: raw.display,
            logging: raw.logging,
            storage: raw.storage,
            theme,
        };

        config.validate()?;
        Ok((config, warnings))
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

        // Validate storage settings
        if let Some(data_dir) = &self.storage.data_dir {
            if data_dir.as_os_str().is_empty() || data_dir.to_string_lossy().trim().is_empty() {
                anyhow::bail!("storage.data_dir must not be empty; remove the key to use the default location");
            }
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

        // `storage.data_dir` is omitted from serialization when unset, so document it as a
        // comment to keep --generate-config a complete reference. The `[storage]` header
        // itself is emitted live (not commented out): an empty `[storage]` table deserializes
        // to the default, and a live header keeps the commented `data_dir` line inside its own
        // section — appending this whole block as a comment after the serialized body would
        // place it after `[theme]`, so uncommenting just `data_dir` would silently land it
        // inside `[theme]` instead, which discards unknown keys instead of failing.
        let storage_docs = "\n\
[storage]\n\
# Directory holding the local SQLite cache. Unset = platform default:\n\
#   Linux    ~/.local/share/terminalist\n\
#   macOS    ~/Library/Application Support/terminalist\n\
#   Windows  %APPDATA%\\terminalist\n\
# A leading ~ is expanded; relative paths resolve against the working directory.\n\
# data_dir = \"/path/to/dir\"\n";

        let full_content = header + &toml_content + storage_docs;

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
