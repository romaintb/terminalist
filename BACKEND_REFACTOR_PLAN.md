# Backend Abstraction & Multi-Backend Support Plan

## Overview
Refactor `sync.rs` (1081 LOC) to be backend-agnostic and support multiple backend instances (Todoist, TickTick, MS Todo, GitHub, etc.) simultaneously.

## Current Problems
- `sync.rs` is tightly coupled to Todoist API
- 1081 lines mixing sync orchestration with backend-specific operations
- No way to support multiple task management backends
- Backend-specific logic scattered throughout the sync service

## Architecture Changes

### 1. Create Backend Trait System (`src/backend/mod.rs`)

**Purpose**: Define common interface all backends must implement

```rust
#[async_trait]
pub trait Backend: Send + Sync {
    // Metadata
    fn backend_type(&self) -> &str;
    fn backend_uuid(&self) -> Uuid;

    // Sync operations
    async fn fetch_projects(&self) -> Result<Vec<CommonProject>, BackendError>;
    async fn fetch_tasks(&self) -> Result<Vec<CommonTask>, BackendError>;
    async fn fetch_labels(&self) -> Result<Vec<CommonLabel>, BackendError>;
    async fn fetch_sections(&self) -> Result<Vec<CommonSection>, BackendError>;

    // CRUD operations
    async fn create_project(&self, args: CreateProjectArgs) -> Result<CommonProject, BackendError>;
    async fn create_task(&self, args: CreateTaskArgs) -> Result<CommonTask, BackendError>;
    async fn create_label(&self, args: CreateLabelArgs) -> Result<CommonLabel, BackendError>;

    async fn update_project(&self, remote_id: &str, args: UpdateProjectArgs) -> Result<CommonProject, BackendError>;
    async fn update_task(&self, remote_id: &str, args: UpdateTaskArgs) -> Result<CommonTask, BackendError>;
    async fn update_label(&self, remote_id: &str, args: UpdateLabelArgs) -> Result<CommonLabel, BackendError>;

    async fn delete_project(&self, remote_id: &str) -> Result<(), BackendError>;
    async fn delete_task(&self, remote_id: &str) -> Result<(), BackendError>;
    async fn delete_label(&self, remote_id: &str) -> Result<(), BackendError>;

    async fn complete_task(&self, remote_id: &str) -> Result<(), BackendError>;
    async fn reopen_task(&self, remote_id: &str) -> Result<(), BackendError>;
}
```

**Common Types** (highest common denominator across backends):
```rust
pub struct CommonProject {
    pub remote_id: String,
    pub name: String,
    pub color: String,
    pub is_favorite: bool,
    pub is_inbox: bool,
    pub order_index: i32,
    pub parent_remote_id: Option<String>,
}

pub struct CommonTask {
    pub remote_id: String,
    pub content: String,
    pub description: Option<String>,
    pub project_remote_id: String,
    pub section_remote_id: Option<String>,
    pub parent_remote_id: Option<String>,
    pub priority: i32,
    pub order_index: i32,
    pub due_date: Option<String>,
    pub due_datetime: Option<String>,
    pub is_recurring: bool,
    pub deadline: Option<String>,
    pub duration: Option<String>,
    pub is_completed: bool,
    pub labels: Vec<String>,
}

pub struct CommonLabel {
    pub remote_id: String,
    pub name: String,
    pub color: String,
    pub order_index: i32,
    pub is_favorite: bool,
}

pub struct CommonSection {
    pub remote_id: String,
    pub name: String,
    pub project_remote_id: String,
    pub order_index: i32,
}
```

**Error Types**:
```rust
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Backend error: {0}")]
    Other(String),
}
```

### 2. Implement TodoistBackend (`src/backend/todoist.rs`)

**Purpose**: Extract all Todoist-specific logic from `sync.rs`

```rust
pub struct TodoistBackend {
    backend_uuid: Uuid,
    wrapper: TodoistWrapper,
}

impl TodoistBackend {
    pub fn new(backend_uuid: Uuid, api_token: String) -> Self {
        Self {
            backend_uuid,
            wrapper: TodoistWrapper::new(api_token),
        }
    }

    // Helper: Transform Todoist API types → Common types
    fn project_to_common(api_project: &todoist_api::Project) -> CommonProject {
        CommonProject {
            remote_id: api_project.id.clone(),
            name: api_project.name.clone(),
            color: api_project.color.clone(),
            is_favorite: api_project.is_favorite,
            is_inbox: api_project.is_inbox_project,
            order_index: api_project.order,
            parent_remote_id: api_project.parent_id.clone(),
        }
    }

    // Similar transformers for tasks, labels, sections...
}

#[async_trait]
impl Backend for TodoistBackend {
    fn backend_type(&self) -> &str { "todoist" }
    fn backend_uuid(&self) -> Uuid { self.backend_uuid }

    async fn fetch_projects(&self) -> Result<Vec<CommonProject>, BackendError> {
        let projects = self.wrapper.get_projects().await
            .map_err(|e| BackendError::Network(e.to_string()))?;
        Ok(projects.iter().map(Self::project_to_common).collect())
    }

    // Implement all other trait methods...
}
```

### 3. Database Schema Updates

**New Entity**: `src/entities/backend.rs`
```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "backends")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub uuid: Uuid,
    pub backend_type: String,  // "todoist", "ticktick", etc.
    pub name: String,           // User-given name like "Work" or "Personal"
    pub config: String,         // JSON-encoded config (tokens, credentials, etc.)
}
```

**Update Existing Entities**: Add to each (project, task, label, section):
```rust
// Add field
pub backend_uuid: Uuid,

// Add relation
#[sea_orm(
    belongs_to = "super::backend::Entity",
    from = "Column::BackendUuid",
    to = "super::backend::Column::Uuid",
    on_delete = "Cascade"
)]
Backend,

// Update unique constraint (in migration or table definition)
// OLD: remote_id UNIQUE
// NEW: (backend_uuid, remote_id) UNIQUE
```

### 4. Refactor SyncService (`src/sync.rs`)

**Before** (tightly coupled):
```rust
pub struct SyncService {
    todoist: TodoistWrapper,  // ❌ Hardcoded to Todoist
    storage: Arc<Mutex<LocalStorage>>,
    sync_in_progress: Arc<Mutex<bool>>,
}
```

**After** (backend-agnostic):
```rust
pub struct SyncService {
    backends: HashMap<Uuid, Box<dyn Backend>>,  // ✅ Multiple backends
    storage: Arc<Mutex<LocalStorage>>,
    sync_in_progress: Arc<Mutex<bool>>,
}

impl SyncService {
    pub async fn new(storage: LocalStorage) -> Result<Self> {
        // Load backends from DB + config
        let backends = BackendRegistry::load_all(&storage).await?;

        Ok(Self {
            backends,
            storage: Arc::new(Mutex::new(storage)),
            sync_in_progress: Arc::new(Mutex::new(false)),
        })
    }

    // Sync ALL backends
    pub async fn sync(&self) -> Result<SyncStatus> {
        for (uuid, backend) in &self.backends {
            self.sync_backend(*uuid, backend.as_ref()).await?;
        }
        Ok(SyncStatus::Success)
    }

    // Sync single backend
    async fn sync_backend(&self, backend_uuid: Uuid, backend: &dyn Backend) -> Result<()> {
        let projects = backend.fetch_projects().await?;
        let tasks = backend.fetch_tasks().await?;
        let labels = backend.fetch_labels().await?;
        let sections = backend.fetch_sections().await?;

        let storage = self.storage.lock().await;
        self.store_projects_batch(&storage, backend_uuid, &projects).await?;
        self.store_tasks_batch(&storage, backend_uuid, &tasks).await?;
        // ...
        Ok(())
    }

    // Create task - lookup which backend owns the project
    pub async fn create_task(&self, content: &str, project_uuid: Uuid) -> Result<()> {
        let storage = self.storage.lock().await;
        let backend_uuid = Self::lookup_project_backend(&storage.conn, &project_uuid).await?;
        drop(storage);

        let backend = self.backends.get(&backend_uuid)
            .ok_or_else(|| anyhow!("Backend not found"))?;

        let common_task = backend.create_task(CreateTaskArgs {
            content: content.to_string(),
            project_remote_id: /* lookup */,
            // ...
        }).await?;

        // Store locally
        self.store_task_locally(backend_uuid, &common_task).await?;
        Ok(())
    }
}
```

### 5. Backend Registry (`src/backend/registry.rs`)

**Purpose**: Factory for creating backend instances

```rust
pub struct BackendRegistry;

impl BackendRegistry {
    /// Load all backends from config + DB
    pub async fn load_all(storage: &LocalStorage) -> Result<HashMap<Uuid, Box<dyn Backend>>> {
        let mut backends = HashMap::new();

        // 1. Load from env var (simple mode - single Todoist)
        if let Ok(token) = std::env::var("TODOIST_API_TOKEN") {
            let uuid = Uuid::new_v4();
            Self::register_backend(storage, uuid, "todoist", "Default", &token).await?;
            let backend = TodoistBackend::new(uuid, token);
            backends.insert(uuid, Box::new(backend) as Box<dyn Backend>);
        }

        // 2. Load from config file (advanced mode - multiple backends)
        if let Ok(config) = Config::load() {
            for backend_config in config.backends {
                let uuid = Uuid::new_v4();
                let backend = Self::create_backend(uuid, &backend_config)?;
                Self::register_backend(
                    storage,
                    uuid,
                    &backend_config.backend_type,
                    &backend_config.name,
                    &backend_config.credentials
                ).await?;
                backends.insert(uuid, backend);
            }
        }

        Ok(backends)
    }

    fn create_backend(uuid: Uuid, config: &BackendConfig) -> Result<Box<dyn Backend>> {
        match config.backend_type.as_str() {
            "todoist" => Ok(Box::new(TodoistBackend::new(uuid, config.api_token.clone()))),
            "ticktick" => Ok(Box::new(TickTickBackend::new(uuid, config.credentials.clone()))),
            _ => Err(anyhow!("Unknown backend type: {}", config.backend_type)),
        }
    }

    async fn register_backend(
        storage: &LocalStorage,
        uuid: Uuid,
        backend_type: &str,
        name: &str,
        config: &str
    ) -> Result<()> {
        // Insert into backends table
        let backend_model = backend::ActiveModel {
            uuid: ActiveValue::Set(uuid),
            backend_type: ActiveValue::Set(backend_type.to_string()),
            name: ActiveValue::Set(name.to_string()),
            config: ActiveValue::Set(config.to_string()),
        };
        backend::Entity::insert(backend_model)
            .exec(&storage.conn)
            .await?;
        Ok(())
    }
}
```

### 6. Configuration Support (`src/config.rs`)

```toml
# Simple mode: just use env var TODOIST_API_TOKEN

# Advanced mode: terminalist.toml
[[backends]]
type = "todoist"
name = "Work"
api_token = "xyz123..."

[[backends]]
type = "ticktick"
name = "Personal"
username = "user@example.com"
password = "..."

[[backends]]
type = "github"
name = "OSS Projects"
token = "ghp_..."
repos = ["owner/repo1", "owner/repo2"]
```

## Implementation Phases

### Phase 1: Foundation (No Breaking Changes)
1. ✅ Create `src/backend/mod.rs` with trait + common types
2. ✅ Create `src/entities/backend.rs`
3. ✅ Update storage to create backends table
4. ✅ Add `backend_uuid` field to all entities (default to single backend for now)

### Phase 2: Extract Todoist
1. ✅ Create `src/backend/todoist.rs`
2. ✅ Implement `Backend` trait for `TodoistBackend`
3. ✅ Copy all Todoist logic from `sync.rs` → `TodoistBackend`

### Phase 3: Refactor Sync
1. ✅ Update `SyncService` to use `HashMap<Uuid, Box<dyn Backend>>`
2. ✅ Make sync loop iterate over all backends
3. ✅ Update CRUD methods to lookup backend_uuid and route to correct backend
4. ✅ Remove Todoist-specific code from `sync.rs`

### Phase 4: Registry & Config
1. ✅ Create `src/backend/registry.rs`
2. ✅ Update `config.rs` to parse backend config
3. ✅ Update initialization flow to use registry

### Phase 5: Testing & Validation
1. ✅ Test single Todoist backend (should work exactly as before)
2. ✅ Add integration tests for multi-backend scenarios
3. ✅ Verify schema constraints (compound unique on remote_id)

## Files to Create
- `src/backend/mod.rs` (~200 lines: trait + common types + errors)
- `src/backend/todoist.rs` (~400 lines: extracted from sync.rs)
- `src/backend/registry.rs` (~150 lines: factory logic)
- `src/entities/backend.rs` (~30 lines: backend entity)

## Files to Modify
- `src/sync.rs` (~500 lines removed, ~200 lines changed - **~50% reduction**)
- `src/entities/project.rs` (+10 lines: backend_uuid field)
- `src/entities/task.rs` (+10 lines: backend_uuid field)
- `src/entities/label.rs` (+10 lines: backend_uuid field)
- `src/entities/section.rs` (+10 lines: backend_uuid field)
- `src/entities/mod.rs` (+1 line: export backend)
- `src/storage.rs` (+5 lines: create backends table)
- `src/config.rs` (+50 lines: backend config parsing)
- `src/lib.rs` (+1 line: export backend module)

## Expected Results

### Before
- ❌ `sync.rs`: 1081 lines, tightly coupled to Todoist
- ❌ No support for multiple backends
- ❌ Backend logic mixed with sync orchestration
- ❌ Hard to add new backends

### After
- ✅ `sync.rs`: ~550 lines, backend-agnostic orchestration
- ✅ `backend/todoist.rs`: ~400 lines, isolated Todoist logic
- ✅ Clear separation of concerns
- ✅ Easy to add new backends (just implement trait)
- ✅ Support for multiple backend instances
- ✅ Same public API (minimal breaking changes)

## Future Extensions (Out of Scope)
- Optimistic updates (update local first, sync later with rollback)
- Cross-backend operations (move task from Todoist → TickTick)
- Backend-specific features (custom fields, advanced features)
- Conflict resolution strategies
- Offline-first sync with delta updates

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Fresh DB at startup | No migration complexity, simpler for now |
| Backend UUID not stable across restarts | Aligns with fresh DB design |
| `remote_id` unique per backend (compound) | Same remote_id can exist in different backends |
| All backends share same local schema | Enforces "highest common denominator" approach |
| Sync all backends in single pass | Simple, can optimize later if needed |
| CRUD operations lookup backend_uuid | Each entity knows its source backend |
| No cross-backend operations | Isolated backends, simpler to reason about |

## Risk Mitigation
- **Risk**: Breaking existing functionality
  - **Mitigation**: Phase 1 keeps current behavior, test at each phase
- **Risk**: Performance degradation with multiple backends
  - **Mitigation**: Parallel sync (future), profile and optimize
- **Risk**: Schema changes break existing code
  - **Mitigation**: Comprehensive search for all entity usage
- **Risk**: Backend trait too rigid for future backends
  - **Mitigation**: Start with common denominator, extend trait as needed
