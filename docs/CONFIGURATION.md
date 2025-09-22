# Configuration Guide

This document explains how to configure Terminalist.

## Configuration Files

Terminalist supports configuration via TOML files. Configuration files are loaded in the following order of precedence:
1. `./terminalist.toml` (project-specific config)
2. `~/.config/terminalist/config.toml` (user config)
3. Built-in defaults

## Generate Default Configuration

```bash
terminalist --generate-config
```

This creates a config file at `~/.config/terminalist/config.toml` with all available options.

## Configuration Options

### Example Configuration

```toml
[ui]
default_project = "today"         # Options: "inbox", "today", "tomorrow", "upcoming", project ID, or project name
mouse_enabled = true              # Enable mouse support
sidebar_width = 30                # Sidebar width in columns (15-50)

[sync]
auto_sync_interval_minutes = 5    # Auto-sync interval (0 = disabled)

[display]
date_format = "%Y-%m-%d"          # Date format for task due dates
time_format = "%H:%M"             # Time format for datetime fields
show_descriptions = true          # Show task descriptions in list view
show_durations = true             # Show task durations
show_labels = true                # Show task labels
show_project_colors = false       # Show project colors

[logging]
enabled = false                   # Enable logging to file
```

### UI Configuration

- **default_project**: Set the initial view when starting the app
  - Options: `"inbox"`, `"today"`, `"tomorrow"`, `"upcoming"`, a specific project ID, or project name
- **mouse_enabled**: Enable or disable mouse support
- **sidebar_width**: Width of the sidebar in columns (must be between 15-50)

### Sync Configuration

- **auto_sync_interval_minutes**: How often to automatically sync with Todoist
  - Set to `0` to disable automatic syncing (manual sync only with `r` key)

### Display Configuration

- **date_format**: Format for displaying dates (uses [chrono format strings](https://docs.rs/chrono/latest/chrono/format/strftime/index.html))
- **time_format**: Format for displaying times
- **show_descriptions**: Whether to show task descriptions in the list view
- **show_durations**: Whether to show task duration information
- **show_labels**: Whether to show task labels as colored badges
- **show_project_colors**: Whether to show project colors

### Logging Configuration

- **enabled**: Enable debug logging to file for troubleshooting