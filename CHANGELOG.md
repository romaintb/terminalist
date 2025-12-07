# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2025-12-07

### Added
- **Backend Abstraction Layer** - Introduced backend entity and registry system with preliminary architectural work for future multi-backend support (Todoist remains the only supported backend and main focus)
- **Repository Pattern** - Implemented repository pattern for clean data access with UUID-based primary keys
- **File-backed SQLite Database** - Added persistent file-based database option to resolve timeout issues
- **Enhanced Scrolling** - Added scrollbars to sidebar and task list with mouse scroll support and visual scroll indicators
- **Clickable Task Selection** - Task list items are now clickable for easier task selection
- **Sidebar Item Component** - Created dedicated component for sidebar items with better abstraction
- **Matrix Chat Channel** - Added Matrix channel for community discussion (#terminalist:matrix.doxin.net)
- **AUR Package** - Official Arch User Repository package with PKGBUILD

### Changed
- **Database Migration to Sea-ORM** - Migrated from SQLx to Sea-ORM for better ORM capabilities and type safety
- **Storage Architecture** - Restructured storage layer with dedicated entity modules and repositories
- **Sync Improvements** - Sections are now synced before tasks to maintain proper hierarchy
- **Dialog System** - Abstracted common dialog patterns into reusable components
- **Logging System** - Enhanced logging with fern v0.7.1 and better in-memory log handling
- **Default Log Level** - Changed default log level to Info for better debugging experience
- **Task Updates** - Task modifications now reflect immediately in the task list
- **Color System** - Removed unused color fields from labels and related entities
- **Sidebar Layout** - Improved sidebar component with better item abstraction and layout
- **Dependency Updates**:
  - sea-orm from 1.1.16 to 1.1.19
  - tokio from 1.47.1 to 1.48.0
  - toml from 0.8.23 to 0.9.7
  - dirs from 5.0.1 to 6.0.0
  - fern from 0.6.2 to 0.7.1
  - serde from 1.0.225 to 1.0.228
  - thiserror from 2.0.16 to 2.0.17
  - anyhow from 1.0.99 to 1.0.100
  - GitHub Actions checkout from v5 to v6

### Fixed
- **Accented Characters** - Fixed crash when using accented characters in task creation modal
- **Sync Order** - Fixed section sync to occur before task sync, preventing hierarchy issues
- **Default Project** - Refreshing data no longer resets view to default project
- **Task Label Relationships** - Fixed and simplified the relationship between tasks and labels
- **Scrollbar Calculations** - Improved scrollbar position calculations and offset handling
- **Sidebar Scrolling** - Fixed sidebar clicking behavior when scrolled
- **Task List Scrollbar** - Fixed task list scrollbar position calculation

### Removed
- **Unused Color Fields** - Removed color field from labels and related entities
- **Custom Color Utilities** - Removed unused color helper utilities
- **Useless Success Dialog** - Removed confirmation dialog that appeared after successful operations
- **Test Files** - Removed obsolete todoist_test.rs and cleaned up unused test utilities

## [0.3.0] - 2025-09-18

### Added
- **Task Search** - Fast database-powered search across all tasks with '/' keyboard shortcut
- **Search Dialog** - VS Code-style command palette for finding tasks with live search results
- **Database Search Optimization** - Search queries run at SQLite level for performance
- Human-readable date formatting - Task due dates now display in Todoist-style format (e.g., "yesterday", "today", "tomorrow", "next Monday")
- Datetime support with time - Tasks with specific times show as "tomorrow at 09:00" instead of raw timestamps
- Comprehensive datetime utilities - New consolidated `datetime.rs` module with robust date parsing and formatting
- **Configuration File Support** - TOML-based configuration system with XDG/platform directory support
- **Screenshot Mode** - Debug mode for injecting test data and generating screenshots
- **Debug Database Backend** - SQLite file backend option for debugging and development
- **Subtask Creation** - Support for creating and managing task hierarchies
- **Upcoming View** - New view for tasks scheduled beyond today
- **Unit Tests** - Comprehensive test suite moved to dedicated tests/ directory
- **Enhanced Documentation** - Split README into multiple focused documents

### Changed
- **Search UI** - Subtle color scheme for search dialog with gray borders and muted project context
- **Task Search Architecture** - Moved from in-memory filtering to efficient database-level queries
- Enhanced task display - Task list items now show intuitive date formatting instead of raw YYYY-MM-DD strings
- Improved code organization - Consolidated date utilities into single module, reducing complexity
- Better datetime parsing - Support for multiple datetime formats (RFC3339, ISO 8601, space-separated)
- **Simplified Text Rendering** - Removed custom ellipsis logic in favor of ratatui's built-in text truncation
- **Cleaner API** - Removed unused `max_width` parameter from `ListItem` trait and implementations
- **Sidebar Layout** - Changed sidebar width from percentage to column count for better control
- **Storage Architecture** - Split storage.rs into focused submodules for better maintainability
- **Task Hierarchy Display** - Prettier indentation and visual hierarchy for subtasks
- **Checkbox UI** - More attractive checkbox rendering in task lists
- **Date Handling** - Use local time consistently instead of UTC throughout the application
- **Configuration Management** - Moved to XDG/platform standard config directories
- **Logging System** - Enhanced file logging with configurable retention limits

### Fixed
- Missing priority key binding - Added "p" key to cycle through task priorities
- SQLite foreign key constraints - Properly enabled foreign key constraints for better data integrity
- Task dialog project selection - Fixed project pre-selection in task creation dialogs
- **Sidebar Text Truncation** - Fixed premature ellipsis truncation in sidebar project and label names
- **Duplicate Subtasks** - Fixed issue where subtasks were displayed twice (once at root level, once at correct hierarchical position)
- **Tomorrow Filter** - Fixed tomorrow's task filtering logic
- **Task Deletion** - Added proper confirmation dialogs for task deletion
- **Labels Storage** - Fixed label saving and display with dedicated database table
- **Today's View** - Corrected database query after storage refactoring
- **Resize Calculations** - Safer sidebar width calculations to prevent UI glitches
- **Date Shortcuts** - Restored t/T/w/W keyboard shortcuts for changing task due dates

### Removed
- **Soft Task Deletion** - Tasks are now permanently deleted instead of marked as completed
- **Task Reopening** - Simplified task lifecycle by removing reopening functionality
- **Migration Mechanism** - Removed unused database migration system
- **Unused Metadata** - Cleaned up unused fields like last_sync and metadata

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