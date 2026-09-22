# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Configurable themes** - Interface colours, background included, come from the config file. See `docs/CONFIGURATION.md`.
- **Periodic auto-sync** - `auto_sync_interval_minutes` is honoured at last. Defaults to 5 minutes, `0` keeps syncing manual.

### Changed
- **Cache kept between runs** - Terminalist opens on the last synced data instead of an empty screen, and a failed sync leaves that data in place.
- **Faster syncing** - Projects, tasks, labels and sections are fetched concurrently.
- **Sync status toast** - The blocking sync dialog became a toast.
- **Rust 1.94 to build from source** - Up from 1.80. Building and packaging only.
- **Smaller build** - Only the tokio and sea-orm features the code uses are compiled.

### Fixed
- **Moving a task to another project** - `Tab` in the edit dialog cycles through the projects, inbox included, and saving moves the task. The key did nothing before.
- **Cursor jumping after a sync** - Task ids are stable across syncs, so the selection holds.
- **Cursor moving on its own** - Neither a background sync nor the first one moves the selection any more.
- **Pipes in task and project names** - A `|` in the text no longer makes creation fail.
- **Tasks missing from a project view** - A task whose section was not loaded now shows with the sectionless ones.
- **Wrong spinner label** - The label matches the operation that is running.
- **Unclear error messages** - Todoist failures keep their own wording and category instead of a generic one.
- **Wrong link for the API token** - The address shown with `TODOIST_API_TOKEN` unset was out of date.
- **Stale cache after an upgrade** - The cache is dropped and rebuilt when the schema version changes.
- **Foreign key constraints** - `foreign_keys` is declared on the pool, so every connection gets it, not only the first one.

### Removed
- **In-repo PKGBUILD** - The Arch package is tracked in its own repository.
- **fern and once_cell dependencies** - No longer used.

### Security
- **Cache file permissions** - The cache holds the API token and is now created `0600`. Existing files are tightened at the next launch.
- **h2 denial of service** - Bumped h2 to 0.4.19 for RUSTSEC-2026-0258.
- **Audit findings cleared** - rustls, lru, event-listener and chacha20 updated; the unreachable rsa advisory is ignored with a note.

## [0.5.0] - 2026-03-25

### Added
- **Togglable Sidebar** - Press `b` to show/hide the sidebar, giving more screen space for the task list

### Changed
- **Todoist API v1** - Migrated from todoist-api 0.3.x to v1.0 with paginated data fetching, ensuring all projects, tasks, labels, and sections are fully retrieved
- **Sync Service Architecture** - Split sync service into focused modules for better maintainability
- **Dependency Updates**:
  - todoist-api from 0.3.1 to 1.0.0-alpha.1
  - tokio from 1.48.0 to 1.49.0
  - serde_json from 1.0.145 to 1.0.149
  - rsa from 0.9.8 to 0.9.10
  - toml from 0.9.8 to 0.9.11
  - thiserror from 2.0.17 to 2.0.18
  - chrono from 0.4.42 to 0.4.43
  - uuid from 1.18.1 to 1.20.0
  - log from 0.4.28 to 0.4.29

### Security
- **bytes integer overflow** - Updated bytes from 1.10.1 to 1.11.1, fixing an integer overflow in `BytesMut::reserve`

### Fixed
- **Dialog Cursor** - Unified cursor handling across all dialogs (task, project, label, search); input fields now use the visible terminal cursor for clearer feedback

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
