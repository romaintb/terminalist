# Architecture Overview

This document describes the technical architecture of Terminalist.

## Project Structure

```
src/
├── main.rs                    # Main application entry point
├── lib.rs                     # Library exports
├── config.rs                  # Configuration management
├── todoist.rs                 # Todoist API models & display structs
├── sync.rs                    # Sync service with API integration
├── storage.rs                 # Storage initialization
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
├── backend_registry.rs        # Backend registry system
├── icons.rs                   # Icon service for terminal compatibility
├── logger.rs                  # Debug logging system
├── utils/                     # Utility modules
│   ├── mod.rs
│   └── datetime.rs            # Date/time utilities
└── ui/                        # Modern Component-Based Architecture
    ├── app_component.rs       # Main application orchestrator
    ├── renderer.rs            # Modern rendering system
    ├── core/                  # Core architecture components
    │   ├── actions.rs         # Action system for component communication
    │   ├── component.rs       # Component trait and lifecycle
    │   ├── context.rs         # App context
    │   ├── event_handler.rs   # Event processing system
    │   └── task_manager.rs    # Background async task management
    └── components/            # UI Components
        ├── badge.rs
        ├── dialog_component.rs    # Unified modal dialog system
        ├── sidebar_component.rs   # Project/label navigation
        ├── task_list_component.rs # Task management and display
        └── task_list_item_component.rs
```

## Data Management

### Local Storage
- Data is cached locally in a **file-backed SQLite database**
- Database is recreated from scratch on each startup by syncing with the backend
- Uses Sea-ORM for type-safe database operations
- Repository pattern provides clean data access layer
- UUID-based primary keys for robust entity management

### Sync Behavior
- **First Run**: Automatically syncs all data from Todoist
- **Startup**: Loads local data instantly, then syncs in background if data is older than 5 minutes
- **Manual Sync**: Press `r` to force refresh from Todoist API
- **Sync Indicators**: Sync progress is shown during operations

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