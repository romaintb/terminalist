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
├── storage/                   # SQLite storage (in-memory)
│   ├── db.rs
│   ├── labels.rs
│   ├── projects.rs
│   ├── sections.rs
│   ├── tasks.rs
│   └── mod.rs
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
- All data is cached locally in an **in-memory SQLite database**
- **No persistence between sessions** - task data is fresh on each launch
- Provides instant loading and fast response times

### Sync Behavior
- **First Run**: Automatically syncs all data from Todoist
- **Startup**: Loads local data instantly, then syncs in background if data is older than 5 minutes
- **Manual Sync**: Press `r` to force refresh from Todoist API
- **Sync Indicators**: Sync progress is shown during operations

### Data Types
- **Projects**: Hierarchical structure with parent-child relationships
- **Tasks**: Full task details including labels, priority, and status
- **Labels**: Colored badges for task categorization
- **Search**: Fast database-level search across all tasks with live results
- **Real-time Updates**: Create, modify, and delete tasks/projects immediately