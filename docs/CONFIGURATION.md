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

[theme]
accent = "Yellow"                 # Selection highlight color for the sidebar's currently-selected entry
success = "Green"                 # Completed-task checkmark icon, save/create dialog actions
danger = "Red"                    # Deleted-task icon/text, delete/cancel shortcuts, error/delete-confirmation dialogs
warning = "Yellow"                # Sync/loading banner, "Edit Project" dialog accent
info = "Cyan"                     # Section/account headers, task & label dialog accent, Tab-select hint
info_dialog = "Blue"              # Border/title accent of the "Info" dialog
project_accent = "Magenta"        # Border/title accent of the "New Project" dialog
project_tag = "Cyan"              # Color of the "#project" tag shown next to a task
due_date = "#FFA500"              # Color of a task's due-date text
label = "Green"                   # Color of "@label" badges
text = "White"                    # Default text color for unselected items and dialog content
text_muted = "DarkGray"           # Descriptions, tree connectors, separators, completed-task text, list scrollbars
border = "DarkGray"               # Color of the sidebar and task-list borders
border_dim = "Gray"               # Dialog chrome borders/scrollbars, child-count badge, instruction separators
selection_bg = "DarkGray"         # Background color of the currently-highlighted row in the task list
```

Note: priority-flag colors (P1-P4) are not configurable. They're a fixed part of Terminalist's
visual language, so they're always red/orange/blue/white regardless of your `[theme]` settings.

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

### Theme Configuration

Each field accepts either a named color (`"Black"`, `"Red"`, `"Green"`, `"Yellow"`, `"Blue"`, `"Magenta"`,
`"Cyan"`, `"Gray"`, `"DarkGray"`, `"LightRed"`, `"LightGreen"`, `"LightYellow"`, `"LightBlue"`,
`"LightMagenta"`, `"LightCyan"`, `"White"`, `"Reset"`) or a hex color (`"#RRGGBB"`). `"Reset"` means "use
the terminal's own default foreground/background" rather than a fixed color.

Colors are semantic — each field is reused everywhere that meaning applies, so changing one field
recolors every matching UI element at once:

- **accent**: Selection highlight color for the sidebar's currently-selected entry
- **success**: Completed-task checkmark icon, and the accent color for save/create actions in dialogs
- **danger**: Deleted-task icon/text, delete/cancel shortcuts, and the error/delete-confirmation dialogs
- **warning**: Sync/loading banner, and the "Edit Project" dialog accent
- **info**: Section/account headers, task & label dialog accent, and the Tab-select hint
- **info_dialog**: Border/title accent of the "Info" dialog
- **project_accent**: Border/title accent of the "New Project" dialog
- **project_tag**: Color of the `#project` tag shown next to a task
- **due_date**: Color of a task's due-date text
- **label**: Color of `@label` badges
- **text**: Default text color for unselected items and dialog content
- **text_muted**: Descriptions, tree connectors, separators, completed-task text, list scrollbars
- **border**: Color of the sidebar and task-list borders
- **border_dim**: Dialog chrome borders/scrollbars, child-count badge, instruction separators
- **selection_bg**: Background color of the currently-highlighted row in the task list

Priority-flag colors (P1-P4) are intentionally not part of `[theme]` — they're a fixed visual
language, so they stay hardcoded regardless of your configuration.

Any field omitted from `[theme]` falls back to its built-in default, so you only need to set the
colors you want to change.

#### Invalid colors

A `[theme]` value that isn't a recognized color (a typo like `"Rde"`, or the wrong type entirely,
like a number) never crashes the app or fails the rest of your config. Only that one field falls
back to its default; everything else in `[theme]` (and the rest of the config file) loads
normally. On startup, Terminalist shows a dialog listing exactly which field(s) fell back and the
line number in your config file where the problem is, for example:

```
⚠ 1 theme color in your config could not be applied and fell back to defaults:

• theme.danger (line 12): 'notacolor' is not a valid color, using default

Fix these in your config file, then restart Terminalist.
```