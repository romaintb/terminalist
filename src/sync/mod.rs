//! Synchronization service module for the terminalist application.
//!
//! This module provides the [`SyncService`] struct which handles data synchronization
//! between remote task management backends and local storage. It manages tasks, projects,
//! labels, and sections, providing both read and write operations with proper error handling
//! and logging.
//!
//! The sync service acts as the main data layer for the application, offering:
//! - Fast local data access for UI operations
//! - Background synchronization with remote backends (Todoist, etc.)
//! - CRUD operations for tasks, projects, and labels
//! - Business logic for special views (Today, Tomorrow, Upcoming)

pub mod labels;
pub mod projects;
pub mod sections;
pub mod storage;
pub mod tasks;

use anyhow::Result;
use log::{error, info};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::storage::LocalStorage;

/// Service that manages data synchronization between remote backends and local storage.
///
/// The `SyncService` acts as the primary data access layer for the application,
/// providing both fast local data retrieval and background synchronization with
/// remote task management backends. It handles all CRUD operations for tasks, projects,
/// labels, and sections while maintaining data consistency between local and remote storage.
///
/// The service uses the backend abstraction layer to support multiple task management
/// services (currently Todoist, with support for more backends planned).
///
/// # Features
/// - Backend-agnostic architecture via trait abstraction
/// - Thread-safe operations using Arc<Mutex<>>
/// - Prevents concurrent sync operations
/// - Provides immediate UI updates after create/update operations
/// - Handles business logic for special views (Today, Tomorrow, Upcoming)
/// - Optional logging support for debugging and monitoring
///
/// # Example
/// ```rust,no_run
/// use terminalist::sync::SyncService;
/// use terminalist::backend_registry::BackendRegistry;
/// use terminalist::storage::LocalStorage;
/// use std::sync::Arc;
/// use tokio::sync::Mutex;
///
/// # async fn example() -> anyhow::Result<()> {
/// let storage = Arc::new(Mutex::new(LocalStorage::new(false).await?));
/// let backend_registry = Arc::new(BackendRegistry::new(storage));
/// // ... initialize and load backends ...
/// # let backend_uuid = uuid::Uuid::new_v4();
/// let sync_service = SyncService::new(backend_registry, backend_uuid, false).await?;
///
/// // Sync data from remote backend
/// sync_service.sync().await?;
///
/// // Get projects from local storage (fast)
/// let projects = sync_service.get_projects().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct SyncService {
    backend_registry: Arc<crate::backend_registry::BackendRegistry>,
    backend_uuid: Uuid,
    storage: Arc<Mutex<LocalStorage>>,
    sync_in_progress: Arc<Mutex<bool>>,
    debug_mode: bool,
}

/// Represents the current status of a synchronization operation.
///
/// This enum is used to communicate the state of sync operations to the UI,
/// allowing for proper status indicators and error handling.
#[derive(Debug, Clone)]
pub enum SyncStatus {
    /// Sync service is not currently performing any operations
    Idle,
    /// A sync operation is currently in progress
    InProgress,
    /// The last sync operation completed successfully
    Success,
    /// The last sync operation failed with an error
    Error {
        /// Human-readable error message describing what went wrong
        message: String,
    },
}

impl SyncService {
    #[cfg(test)]
    pub(crate) fn new_for_test(storage: Arc<Mutex<LocalStorage>>, backend_uuid: Uuid) -> Self {
        Self {
            backend_registry: Arc::new(crate::backend_registry::BackendRegistry::new(storage.clone())),
            backend_uuid,
            storage,
            sync_in_progress: Arc::new(Mutex::new(false)),
            debug_mode: false,
        }
    }

    /// Creates a new `SyncService` instance with the provided backend registry.
    ///
    /// This creates a sync service that manages synchronization for a specific backend.
    /// The backend instance is retrieved from the registry on-demand.
    ///
    /// # Arguments
    /// * `backend_registry` - Shared backend registry instance
    /// * `backend_uuid` - UUID of the backend this service will manage
    /// * `debug_mode` - Whether to enable debug mode for local storage
    ///
    /// # Returns
    /// A new `SyncService` instance ready for use
    ///
    /// # Errors
    /// Returns an error if the backend UUID is not found in the registry
    pub async fn new(
        backend_registry: Arc<crate::backend_registry::BackendRegistry>,
        backend_uuid: Uuid,
        debug_mode: bool,
    ) -> Result<Self> {
        // Verify backend exists
        backend_registry.get_backend(&backend_uuid).await?;

        let storage = backend_registry.storage();

        Ok(Self {
            backend_registry,
            backend_uuid,
            storage,
            sync_in_progress: Arc::new(Mutex::new(false)),
            debug_mode,
        })
    }

    /// Helper to get the current backend instance from the registry.
    async fn get_backend(&self) -> Result<Arc<Box<dyn crate::backend::Backend>>> {
        self.backend_registry.get_backend(&self.backend_uuid).await
    }

    /// Returns whether debug mode is enabled.
    ///
    /// This is used to enable debug-only features like local data refresh.
    pub fn is_debug_mode(&self) -> bool {
        self.debug_mode
    }

    /// Checks if a synchronization operation is currently in progress.
    ///
    /// This method is useful for UI components to show loading indicators
    /// and prevent concurrent sync operations.
    ///
    /// # Returns
    /// `true` if sync is in progress, `false` otherwise
    pub async fn is_syncing(&self) -> bool {
        *self.sync_in_progress.lock().await
    }

    /// Performs a full synchronization with the remote backend.
    ///
    /// This method fetches all projects, tasks, labels, and sections from the remote backend
    /// and stores them in local storage. It ensures that only one sync operation can run
    /// at a time to prevent data corruption and resource conflicts.
    ///
    /// The sync process includes:
    /// 1. Fetching projects, tasks, labels, and sections from the remote backend
    /// 2. Storing all data in local storage with proper ordering
    /// 3. Handling backend errors gracefully with detailed error messages
    /// 4. Providing progress logging for debugging and monitoring
    ///
    /// # Returns
    /// A `SyncStatus` indicating the result of the sync operation
    ///
    /// # Errors
    /// Returns `SyncStatus::Error` if any part of the sync process fails
    pub async fn sync(&self) -> Result<SyncStatus> {
        // Check if sync is already in progress and acquire lock
        let mut sync_guard = self.sync_in_progress.lock().await;
        if *sync_guard {
            return Ok(SyncStatus::InProgress);
        }
        *sync_guard = true;

        // Release the lock before performing sync to avoid holding it during the long operation
        drop(sync_guard);

        let result = self.perform_sync().await;

        // Release sync lock
        {
            let mut sync_guard = self.sync_in_progress.lock().await;
            *sync_guard = false;
        }

        result
    }

    /// Internal sync implementation
    async fn perform_sync(&self) -> Result<SyncStatus> {
        info!("🔄 Starting sync process...");

        // One backend, four requests in flight at once. Each arm below keeps its own error
        // handling, sections included: it degrades to an empty Vec instead of failing the sync.
        let backend = self.get_backend().await?;
        let (projects, tasks, labels, sections) = tokio::join!(
            backend.fetch_projects(),
            backend.fetch_tasks(),
            backend.fetch_labels(),
            backend.fetch_sections(),
        );

        let projects = match projects {
            Ok(projects) => {
                info!("✅ Fetched {} projects from backend", projects.len());
                projects
            }
            Err(e) => {
                error!("❌ Failed to fetch projects: {e}");
                return Ok(SyncStatus::Error {
                    message: format!("Failed to fetch projects: {e}"),
                });
            }
        };

        let tasks = match tasks {
            Ok(tasks) => {
                info!("✅ Fetched {} tasks from backend", tasks.len());
                tasks
            }
            Err(e) => {
                error!("❌ Failed to fetch tasks: {e}");
                return Ok(SyncStatus::Error {
                    message: format!("Failed to fetch tasks: {e}"),
                });
            }
        };

        let labels = match labels {
            Ok(labels) => {
                info!("✅ Fetched {} labels from backend", labels.len());
                labels
            }
            Err(e) => {
                error!("❌ Failed to fetch labels: {e}");
                return Ok(SyncStatus::Error {
                    message: format!("Failed to fetch labels: {e}"),
                });
            }
        };

        let sections = match sections {
            Ok(sections) => {
                info!("✅ Fetched {} sections from backend", sections.len());
                sections
            }
            Err(e) => {
                error!("❌ Failed to fetch sections: {e}");
                info!("⚠️  Skipping sections sync due to backend compatibility issue");
                // For now, skip sections sync and continue with other data
                Vec::new()
            }
        };

        // Store in local database
        {
            let storage = self.storage.lock().await;
            info!("💾 Storing data in local database...");

            if let Err(e) = self.store_snapshot(&storage, &projects, &labels, &sections, &tasks).await {
                error!("❌ Failed to refresh local cache: {e:#}");
                return Ok(SyncStatus::Error {
                    message: format!("Failed to refresh local cache: {e:#}"),
                });
            }
            info!("✅ Refreshed local cache transactionally");
        }

        Ok(SyncStatus::Success)
    }

    /// Forces a full synchronization with the remote backend, bypassing any checks (e.g., last sync time).
    ///
    /// This method is intended for situations where an immediate and complete synchronization
    /// is required, regardless of when the last sync occurred or whether changes are detected.
    /// Currently, it defers to the `sync` method, but it is designed for future enhancements
    /// where it might, for example, clear local cache before syncing or bypass rate limiting logic.
    ///
    /// # Returns
    /// A `SyncStatus` indicating the result of the force sync operation.
    ///
    /// # Errors
    /// Returns `SyncStatus::Error` if any part of the sync process fails.
    pub async fn force_sync(&self) -> Result<SyncStatus> {
        self.sync().await
    }
}
