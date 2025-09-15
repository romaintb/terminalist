# Config File Feature Specification

## Overview

This document outlines the planned configuration file feature for Terminalist, a TUI application for Todoist. The config file will allow users to customize various aspects of the application behavior, appearance, and functionality without modifying code or environment variables.

## Motivation

Currently, Terminalist relies on:
- Environment variables for API token (`TODOIST_API_TOKEN`)
- Command-line flags for debug mode (`--debug`)
- Hardcoded defaults for all other settings

A configuration file will provide:
- **Centralized settings management** - All preferences in one place
- **Enhanced user experience** - Customizable behavior without recompilation
- **Better defaults** - Project-specific or user-specific configurations
- **Reduced friction** - No need to set environment variables repeatedly

## Configuration File Format

### Format: TOML
- Human-readable and editable
- Good support for nested structures
- Widely used in Rust ecosystem
- Easy to parse and validate

## Configuration File Location

### Default Locations (in order of precedence):
1. `./terminalist.toml` (project-specific config)
2. `~/.config/terminalist/config.toml` (user config)
3. Built-in defaults

### Environment Override:
- `TERMINALIST_CONFIG_PATH` - Override config file location

## Configuration File Generation

### Generating Default Config

Users can generate a default configuration file using the `--generate-config` flag:

```bash
terminalist --generate-config
```

This will:
1. Create a `terminalist.toml` file in the current directory
2. Include all available configuration options with their default values
3. Include helpful comments explaining each setting
4. Exit after creating the file (does not start the application)

### Example Generated Config

```toml
# Terminalist Configuration File
# Generated on 2025-01-27

[ui]
# Default project to open on startup
# Options: "inbox", "today", "tomorrow", "upcoming", or project ID
default_project = "today"

# Enable mouse support
mouse_enabled = true

# Sidebar width percentage (10-40)
sidebar_width = 25

[sync]
# Auto-sync interval in minutes (0 = disabled, manual sync only)
auto_sync_interval_minutes = 5

[display]
# Date format for task due dates
date_format = "%Y-%m-%d"

# Time format for datetime fields
time_format = "%H:%M"

# Show task descriptions in list view
show_descriptions = true

# Show task durations
show_durations = true

# Show task labels
show_labels = true

# Show project colors
show_project_colors = false

[logging]
# Log to file
enabled = false
```

## Configuration Sections

### 1. API Configuration

```toml
[api]
# API request timeout in seconds
timeout = 30

# Enable API request logging
debug_requests = false

# Note: API token remains in TODOIST_API_TOKEN environment variable for security
```

### 2. UI Configuration

```toml
[ui]
# Default project to open on startup
# Options: "inbox", "today", "tomorrow", "upcoming", or project ID
default_project = "today"

# Enable mouse support
mouse_enabled = true

# Sidebar width percentage (10-40)
sidebar_width = 25
```

### 3. Sync Configuration

```toml
[sync]
# Auto-sync interval in minutes (0 = disabled, manual sync only)
auto_sync_interval_minutes = 5
```

# Note: Keyboard shortcuts are not configurable in this version

### 4. Display Configuration

```toml
[display]
# Date format for task due dates
date_format = "%Y-%m-%d"

# Time format for datetime fields
time_format = "%H:%M"

# Show task descriptions in list view
show_descriptions = true

# Show task durations
show_durations = true

# Show task labels
show_labels = true

# Show project colors
show_project_colors = false
```

# Note: Theme configuration will be added in a future version

### 5. Logging Configuration

```toml
[logging]
# Enable logging
enabled = false
```

## Default Configuration

The application will ship with sensible defaults that work for most users:

```toml
# Default terminalist.toml
[api]
timeout = 30
debug_requests = false

[ui]
default_project = "today"
mouse_enabled = true
sidebar_width = 25

[sync]
auto_sync_interval_minutes = 5

[display]
date_format = "%Y-%m-%d"
time_format = "%H:%M"
show_descriptions = true
show_durations = true
show_labels = true
show_project_colors = false

[logging]
enabled = false
```

## Implementation Plan

### Phase 1: Basic Configuration Support
1. **Config file parsing** - Add TOML parsing with `toml` crate
2. **Config structure** - Define Rust structs for all configuration sections
3. **File discovery** - Implement config file location resolution
4. **Environment override** - Support `TERMINALIST_CONFIG_PATH` environment variable
5. **Basic settings** - Implement API timeout, default project, and UI settings
6. **Config generation** - Add `--generate-config` flag to create default config file

### Phase 2: Core Features
1. **Sync configuration** - Auto-sync intervals, retry logic, notifications
2. **Display options** - Date formats, priority styles, completion styles
3. **Logging configuration** - File logging, log levels, rotation
4. **Validation** - Config file validation and error reporting

### Phase 3: User Experience
1. **Config validation** - Show errors and exit on invalid config
2. **Config documentation** - Built-in help for configuration options
3. **Error handling** - Clear error messages for config issues

### Future Phases
1. **Theme support** - Color schemes and custom colors
2. **Keyboard shortcuts** - Configurable keybindings system
3. **Performance tuning** - Expose performance-related settings
4. **Config editor** - Optional TUI config editor

## Migration Strategy

### From Environment Variables
- `TODOIST_API_TOKEN` → Remains as environment variable for security
- `--debug` flag → Remains as command-line flag (database settings not exposed)

### XDG Base Directory Compliance
- User config: `~/.config/terminalist/config.toml` (follows XDG Base Directory Specification)
- Log files: `~/.config/terminalist/terminalist.log` (when enabled)

### Backward Compatibility
- Environment variables will continue to work
- Config file settings override hardcoded defaults
- Graceful fallback to defaults if config file is invalid

## Error Handling

### Config File Issues
- **Missing file**: Use defaults, no message (normal behavior)
- **Invalid syntax**: Show error and exit
- **Invalid values**: Show specific error and exit
- **Permission issues**: Show error and exit

### Validation
- **Range validation**: Numeric values within reasonable bounds
- **Enum validation**: String values must match allowed options
- **Path validation**: File paths must be valid and accessible

## Testing Strategy

### Unit Tests
- Config parsing and validation
- Default value handling
- Environment variable override
- Error handling scenarios

### Integration Tests
- Config file discovery
- Settings application to application behavior
- Migration from environment variables
- Performance impact of config loading

### User Testing
- Config file usability
- Default settings appropriateness
- Error message clarity
- Documentation completeness

## Future Enhancements

### Advanced Features
1. **Config profiles** - Multiple named configurations
2. **Config inheritance** - Project-specific configs inherit from user config
3. **Config templates** - Predefined configs for different use cases
4. **Config sharing** - Export/import configurations
5. **Config validation** - Real-time validation in config editor
6. **Config migration** - Automatic migration between versions

### Integration Features
1. **Config sync** - Sync config across devices via Todoist
2. **Config backup** - Automatic config backup and restore
3. **Config versioning** - Track config changes over time
4. **Config analytics** - Anonymous usage statistics for config improvements

## Conclusion

The configuration file feature will significantly improve Terminalist's usability and customization options. By providing a comprehensive, well-structured configuration system, users will be able to tailor the application to their specific needs and workflows.

The phased implementation approach ensures that the feature can be delivered incrementally while maintaining application stability and user experience quality.
