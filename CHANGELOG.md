# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Task Search** - Fast database-powered search across all tasks with '/' keyboard shortcut
- **Search Dialog** - VS Code-style command palette for finding tasks with live search results
- **Database Search Optimization** - Search queries run at SQLite level for performance
- Human-readable date formatting - Task due dates now display in Todoist-style format (e.g., "yesterday", "today", "tomorrow", "next Monday")
- Datetime support with time - Tasks with specific times show as "tomorrow at 09:00" instead of raw timestamps
- Comprehensive datetime utilities - New consolidated `datetime.rs` module with robust date parsing and formatting

### Changed
- **Search UI** - Subtle color scheme for search dialog with gray borders and muted project context
- **Task Search Architecture** - Moved from in-memory filtering to efficient database-level queries
- Enhanced task display - Task list items now show intuitive date formatting instead of raw YYYY-MM-DD strings
- Improved code organization - Consolidated date utilities into single module, reducing complexity
- Better datetime parsing - Support for multiple datetime formats (RFC3339, ISO 8601, space-separated)
- **Simplified Text Rendering** - Removed custom ellipsis logic in favor of ratatui's built-in text truncation
- **Cleaner API** - Removed unused `max_width` parameter from `ListItem` trait and implementations

### Fixed
- Missing priority key binding - Added "p" key to cycle through task priorities
- SQLite foreign key constraints - Properly enabled foreign key constraints for better data integrity
- Task dialog project selection - Fixed project pre-selection in task creation dialogs
- **Sidebar Text Truncation** - Fixed premature ellipsis truncation in sidebar project and label names
- **Duplicate Subtasks** - Fixed issue where subtasks were displayed twice (once at root level, once at correct hierarchical position)
- **Tomorrow Filter** - Fixed tomorrow's task filtering logic

## [0.2.0] - 2025-09-11

### Added
- Task creation dialog now shows sub-projects for better project organization
- Color helper to match Todoist color names
- README badges for better project visibility

### Changed
- Upgraded todoist-api dependency to v0.3.0
- Simplified tasks list box title rendering
- Renamed new_render to simply renderer for cleaner code structure

### Fixed
- Task creation dialog properly pre-selects the current project as task's project
- Linting errors and missing newlines

### Removed
- Removed mentions of the old status bar
- Cleaned up unused/dead code
- Removed traces of the old statusbar implementation