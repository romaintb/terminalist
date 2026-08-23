# Architecture Overview

This document describes the technical architecture of Terminalist.

## Project Structure

```
src/
├── main.rs                    # Main application entry point
├── lib.rs                     # Library exports
├── config.rs                  # Configuration management
├── constants.rs               # Shared UI text and other constants
├── theme.rs                   # Semantic color theme configuration
├── todoist.rs                 # Todoist API models & display structs
├── storage.rs                 # Persistent SQLite cache initialization
├── sync/                      # Sync service with API integration
│   ├── mod.rs                 # SyncService and shared sync logic
│   ├── storage.rs             # Reconciliation (upsert + delete-missing)
│   ├── projects.rs
│   ├── sections.rs
│   ├── tasks.rs
│   └── labels.rs
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
├── backend_registry.rs        # Backend registry (derives a stable UUID)
├── icons.rs                   # Icon service for terminal compatibility
├── logger.rs                  # Debug logging system
├── utils/                     # Utility modules
│   ├── mod.rs
│   └── datetime.rs            # Date/time utilities
└── ui/                        # Modern Component-Based Architecture
    ├── mod.rs
    ├── app_component.rs       # Main application orchestrator
    ├── renderer.rs            # Modern rendering system
    ├── layout.rs              # Layout calculations
    ├── core/                  # Core architecture components
    │   ├── actions.rs         # Action system for component communication
    │   ├── component.rs       # Component trait and lifecycle
    │   ├── context.rs         # App context
    │   ├── event_handler.rs   # Event processing system
    │   ├── task_manager.rs    # Background async task management
    │   └── mod.rs
    └── components/            # UI Components
        ├── badge.rs
        ├── dialog_component.rs    # Unified modal dialog system
        ├── dialogs/               # Per-entity dialog content
        ├── sidebar_component.rs   # Project/label navigation
        ├── sidebar_item_component.rs
        ├── scrollbar_helper.rs
        ├── sync_toast.rs          # Non-blocking sync status toast
        ├── task_list_component.rs # Task management and display
        ├── task_list_item_component.rs
        └── mod.rs
```

## Data Management

### Local Storage
- Data is cached locally in a **file-backed SQLite database** that persists
  across launches — it is opened (or created) rather than deleted and rebuilt
- A backend row's UUID is **derived** from `(backend_type, name)` via
  `Uuid::new_v5` when the row is new, and **adopted** from the existing row
  when one is already there — including the random UUID written by versions
  predating the derived scheme. Either way a relaunch resolves to the same
  row, so the cache keyed to it is never orphaned or duplicated
- Sync **reconciles** the cache instead of replacing it: each entity type is
  upserted on `(backend_uuid, remote_id)`, then any local row whose
  `remote_id` the remote no longer returned is deleted. Local UUIDs stay
  stable across syncs. An **empty** fetch is the one exception: it is treated
  as "nothing to reconcile" rather than "the remote has nothing", so an
  empty-but-successful response cannot blank the cache
- Uses Sea-ORM for type-safe database operations
- Repository pattern provides clean data access layer
- UUID-based primary keys for robust entity management

### Sync Behavior
- **Startup**: Cached data is loaded and painted immediately, and a sync
  with the backend starts in the background without blocking the UI
- **Concurrent Fetch**: Projects, tasks, labels, and sections are fetched
  from the backend concurrently (`tokio::join!`) rather than one after
  another
- **Auto Sync**: After the first sync of the session, a background sync
  fires again once `auto_sync_interval_minutes` has elapsed since the last
  one; set it to `0` to disable auto-sync entirely
- **Manual Sync**: Press `r` to force refresh from Todoist API at any time
- **Sync Indicators**: A toast in the lower-right corner of the task list
  shows syncing/success/failure status without blocking interaction

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