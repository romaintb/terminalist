//! Configuration management for Terminalist
//!
//! This module handles loading, parsing, and validation of configuration files.

use crate::constants::{CONFIG_GENERATED, SIDEBAR_DEFAULT_WIDTH, SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH};
use crate::utils::datetime;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    /// Map of backend_id -> backend configuration
    /// This allows multiple instances of the same backend type
    pub instances: HashMap<String, BackendInstanceConfig>,
}

/// Configuration for a single backend instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendInstanceConfig {
    /// Backend type (e.g., "todoist", "caldav", "local")
    pub backend_type: String,
    /// Human-readable name for this backend instance
    pub name: String,
    /// Whether this backend instance is enabled
    pub enabled: bool,
    /// Backend-specific configuration as a map of key-value pairs
    pub config: HashMap<String, String>,
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
        let mut instances = HashMap::new();

        // Create default Todoist backend instance
        instances.insert(
            "todoist".to_string(),
            BackendInstanceConfig {
                backend_type: "todoist".to_string(),
                name: "Todoist".to_string(),
                enabled: true,
                config: {
                    let mut config = HashMap::new();
                    config.insert("api_token_env".to_string(), "TODOIST_API_TOKEN".to_string());
                    config
                },
            },
        );

        Self {
            default_backend: "todoist".to_string(),
            instances,
        }
    }
}

impl BackendInstanceConfig {
    /// Create a new Todoist backend instance configuration
    pub fn new_todoist(_backend_id: String, name: String, api_token_env: String) -> Self {
        let mut config = HashMap::new();
        config.insert("api_token_env".to_string(), api_token_env);

        Self {
            backend_type: "todoist".to_string(),
            name,
            enabled: true,
            config,
        }
    }

    /// Get a configuration value by key
    pub fn get_config(&self, key: &str) -> Option<&String> {
        self.config.get(key)
    }

    /// Get the API token environment variable for Todoist backends
    pub fn get_api_token_env(&self) -> Option<&String> {
        if self.backend_type == "todoist" {
            self.get_config("api_token_env")
        } else {
            None
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
        // Check if default backend exists and is enabled
        let default_backend = &self.backends.default_backend;
        match self.backends.instances.get(default_backend) {
            Some(instance) => {
                if !instance.enabled {
                    anyhow::bail!("default_backend '{}' is disabled", default_backend);
                }
            }
            None => {
                let available: Vec<String> = self.get_available_backend_ids();
                anyhow::bail!(
                    "default_backend '{}' not found. Available backends: {}",
                    default_backend,
                    if available.is_empty() { "none".to_string() } else { available.join(", ") }
                );
            }
        }

        // Validate each backend instance
        for (backend_id, instance) in &self.backends.instances {
            if instance.enabled {
                self.validate_backend_instance(backend_id, instance)?;
            }
        }

        Ok(())
    }

    /// Validate a single backend instance
    fn validate_backend_instance(&self, backend_id: &str, instance: &BackendInstanceConfig) -> Result<()> {
        // Validate common fields
        if instance.name.is_empty() {
            anyhow::bail!("Backend '{}': name cannot be empty", backend_id);
        }
        if instance.backend_type.is_empty() {
            anyhow::bail!("Backend '{}': backend_type cannot be empty", backend_id);
        }

        // Validate backend-specific configuration
        match instance.backend_type.as_str() {
            "todoist" => {
                if let Some(api_token_env) = instance.get_config("api_token_env") {
                    if api_token_env.is_empty() {
                        anyhow::bail!("Backend '{}': api_token_env cannot be empty", backend_id);
                    }
                } else {
                    anyhow::bail!("Backend '{}': missing required config 'api_token_env'", backend_id);
                }
            }
            backend_type => {
                anyhow::bail!("Backend '{}': unsupported backend_type '{}'", backend_id, backend_type);
            }
        }

        Ok(())
    }

    /// Get list of available (enabled) backend IDs
    pub fn get_available_backend_ids(&self) -> Vec<String> {
        self.backends
            .instances
            .iter()
            .filter(|(_, instance)| instance.enabled)
            .map(|(backend_id, _)| backend_id.clone())
            .collect()
    }

    /// Get all backend instances (enabled and disabled)
    pub fn get_all_backend_instances(&self) -> &HashMap<String, BackendInstanceConfig> {
        &self.backends.instances
    }

    /// Get a specific backend instance configuration
    pub fn get_backend_instance(&self, backend_id: &str) -> Option<&BackendInstanceConfig> {
        self.backends.instances.get(backend_id)
    }

    /// Check if a specific backend instance is enabled
    pub fn is_backend_enabled(&self, backend_id: &str) -> bool {
        self.backends
            .instances
            .get(backend_id)
            .map(|instance| instance.enabled)
            .unwrap_or(false)
    }

    /// Get backend instances by type
    pub fn get_backends_by_type(&self, backend_type: &str) -> Vec<(&String, &BackendInstanceConfig)> {
        self.backends
            .instances
            .iter()
            .filter(|(_, instance)| instance.backend_type == backend_type && instance.enabled)
            .collect()
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
