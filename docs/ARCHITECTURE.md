# Architecture Overview

This document describes the technical architecture of Terminalist.

## Project Structure

```
src/
├── main.rs                    # Main application entry point
├── lib.rs                     # Library exports
├── config.rs                  # Configuration management
├── constants.rs               # Shared constants and UI strings
├── theme.rs                   # Theme colors and config parsing
├── storage.rs                 # SQLite cache: connection, schema, schema versioning
├── icons.rs                   # Icon glyphs rendered in the UI
├── logger.rs                  # Logging setup
├── backend_registry.rs        # Backend registry system
├── entities/                  # Sea-ORM domain entities
│   ├── backend.rs             # Backend entity (Todoist, etc.)
│   ├── label.rs
│   ├── project.rs
│   ├── section.rs
│   ├── task.rs
│   ├── task_label.rs
│   └── mod.rs
├── repositories/              # Repository pattern for data access
│   ├── backend.rs
│   ├── label.rs
│   ├── project.rs
│   ├── section.rs
│   ├── task.rs
│   └── mod.rs
├── backend/                   # Backend abstraction layer
│   ├── factory.rs
│   ├── todoist.rs             # Todoist backend implementation
│   └── mod.rs
├── sync/                      # Sync service, one module per entity type
│   ├── labels.rs
│   ├── projects.rs
│   ├── sections.rs
│   ├── tasks.rs
│   ├── storage.rs             # Reconciles fetched data into the cache
│   └── mod.rs
├── utils/                     # Utility modules
│   ├── mod.rs
│   └── datetime.rs            # Date/time utilities
└── ui/                        # Component-based architecture
    ├── app_component/         # Main application orchestrator
    │   ├── actions.rs         # Action handling
    │   ├── keys.rs            # Global key bindings
    │   ├── state.rs           # Application state
    │   └── mod.rs
    ├── renderer.rs            # Rendering system
    ├── layout.rs              # Layout management
    ├── core/                  # Core architecture components
    │   ├── actions.rs         # Action system for component communication
    │   ├── component.rs       # Component trait and lifecycle
    │   ├── event_handler.rs   # Event processing system
    │   ├── operations.rs      # Background operation descriptions
    │   ├── task_manager.rs    # Background async task management
    │   └── mod.rs
    └── components/            # UI Components
        ├── badge.rs
        ├── dialog_component.rs        # Unified modal dialog system
        ├── dialogs/                   # Per-domain dialog rendering
        │   ├── common.rs
        │   ├── label_dialogs.rs
        │   ├── project_dialogs.rs
        │   ├── scroll_behavior.rs
        │   ├── system_dialogs.rs      # Help, logs, info, error
        │   ├── task_dialogs.rs
        │   └── mod.rs
        ├── scrollbar_helper.rs
        ├── sidebar_component.rs       # Project/label navigation
        ├── sidebar_item_component.rs
        ├── task_list_component.rs     # Task management and display
        ├── task_list_item_component.rs
        ├── toast.rs                   # Corner sync/status toast
        └── mod.rs
```

## Data Management

### Local Storage
- Data is cached locally in a **file-backed SQLite database**
- Database persists between runs and is treated as a disposable cache; the backend stays authoritative
- The schema revision is stamped in `PRAGMA user_version`; a mismatch drops and rebuilds every table rather than migrating
- Uses Sea-ORM for type-safe database operations
- Repository pattern provides clean data access layer
- UUID-based primary keys for robust entity management
- The file lives in the XDG data directory (`terminalist/terminalist.db`) and holds the API token, so it is created `0600` on Unix

### Sync Behavior
- **First Run**: Automatically syncs all data from Todoist
- **Startup**: Shows the cached data immediately, then syncs in the background; a failed sync leaves the cached view in place
- **Periodic Sync**: Re-syncs every `auto_sync_interval_minutes` (5 by default, `0` disables it)
- **Manual Sync**: Press `r` to force refresh from Todoist API
- **Concurrent Fetch**: Projects, tasks, labels and sections are fetched in parallel
- **Sync Indicators**: Sync progress is shown in a corner toast that does not block the interface
- **Debug Mode**: `--debug` skips the initial and periodic syncs, so the cache can be inspected as-is. `r` still syncs on demand, and `R` reloads the view from the cache.

### Data Types
- **Backends**: Abstract backend entity supporting multiple task management services (Todoist, etc.)
- **Projects**: Hierarchical structure with parent-child relationships
- **Sections**: Project sections for organizing tasks
- **Tasks**: Full task details including labels, priority, and status
- **Labels**: Colored badges for task categorization
- **Search**: Fast database-level search across all tasks with live results
- **Real-time Updates**: Create, modify, and delete tasks/projects immediately

### Backend Abstraction
- **Backend Registry**: Centralized system for managing multiple backend services
- **Repository Pattern**: Clean separation between data access and business logic
- **Entity System**: Sea-ORM entities with UUID primary keys and backend associations
- **Current Status**: Todoist is the only supported backend and remains the main focus. Preliminary architectural work has been completed to enable future support for other task management services.