use terminalist::config::Config;
use terminalist::utils::datetime;

#[test]
fn test_default_config() {
    let config = Config::default();
    assert_eq!(config.ui.default_project, "today");
    assert_eq!(config.sync.auto_sync_interval_minutes, 5);
    assert!(config.display.show_descriptions);
    assert!(!config.display.show_project_colors);
    assert!(!config.logging.enabled);
}

#[test]
fn test_config_validation() {
    let mut config = Config::default();

    // Valid config should pass
    assert!(config.validate().is_ok());

    // Invalid sidebar width should fail
    config.ui.sidebar_width = 10;
    assert!(config.validate().is_err());

    // Reset and test invalid sync interval
    config.ui.sidebar_width = 35;
    config.sync.auto_sync_interval_minutes = 2000;
    assert!(config.validate().is_err());
}

#[test]
fn test_config_serialization() {
    let config = Config::default();
    let toml_str = toml::to_string_pretty(&config).unwrap();
    assert!(toml_str.contains("default_project = \"today\""));
    assert!(toml_str.contains("auto_sync_interval_minutes = 5"));
}

#[test]
fn test_partial_config_deserialization() {
    // Test that partial TOML configs merge with defaults
    let partial_toml = r#"
[ui]
sidebar_width = 35

[logging]
enabled = true
"#;

    let config: Config = toml::from_str(partial_toml).unwrap();

    // Check that specified values are used
    assert_eq!(config.ui.sidebar_width, 35);
    assert!(config.logging.enabled);

    // Check that unspecified values use defaults
    assert_eq!(config.ui.default_project, "today"); // default value
    assert!(config.ui.mouse_enabled); // default value
    assert_eq!(config.sync.auto_sync_interval_minutes, 5); // default value
    assert_eq!(config.display.date_format, datetime::TODOIST_DATE_FORMAT); // default value
    assert!(config.display.show_descriptions); // default value
    assert!(!config.display.show_project_colors); // default value
}

#[test]
fn test_empty_config_deserialization() {
    // Test that empty TOML uses all defaults
    let empty_toml = "";
    let config: Config = toml::from_str(empty_toml).unwrap();
    let default_config = Config::default();

    assert_eq!(config.ui.default_project, default_config.ui.default_project);
    assert_eq!(
        config.sync.auto_sync_interval_minutes,
        default_config.sync.auto_sync_interval_minutes
    );
    assert_eq!(config.logging.enabled, default_config.logging.enabled);
    assert_eq!(config.display.date_format, default_config.display.date_format);
}

#[test]
fn test_generate_config_creates_directory() {
    use std::fs;

    // Create a temporary path that doesn't exist
    let temp_dir = std::env::temp_dir().join("terminalist_test_config");
    let config_path = temp_dir.join("nested").join("config.toml");

    // Ensure the directory doesn't exist initially
    if temp_dir.exists() {
        let _ = fs::remove_dir_all(&temp_dir);
    }
    assert!(!temp_dir.exists());

    // Generate config should create the directory structure
    let result = Config::generate_default_config(&config_path);
    assert!(result.is_ok());

    // Verify the directory was created
    assert!(temp_dir.exists());
    assert!(config_path.parent().unwrap().exists());
    assert!(config_path.exists());

    // Verify the file contains expected content
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("# Terminalist Configuration File"));
    assert!(content.contains("default_project = \"today\""));

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}
