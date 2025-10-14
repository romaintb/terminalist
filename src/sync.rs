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

use anyhow::Result;
use log::{error, info};
use sea_orm::{ActiveValue, EntityTrait, IntoActiveModel, TransactionTrait};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::entities::{label, project, section, task, task_label};
use crate::repositories::{LabelRepository, ProjectRepository, SectionRepository, TaskRepository};
use crate::storage::LocalStorage;
use crate::utils::datetime;

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

    /// Retrieves all projects from local storage.
    ///
    /// This method provides fast access to cached project data without making backend calls.
    /// Projects are sorted and ready for display in the UI.
    ///
    /// # Returns
    /// A vector of `project::Model` objects representing all available projects
    ///
    /// # Note
    /// As of 2025, Todoist allows free plan users to create more than 5 projects via the backend,
    /// but the GET /projects backend endpoint will only return the first 5 projects for free users.
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_projects(&self) -> Result<Vec<project::Model>> {
        let storage = self.storage.lock().await;
        ProjectRepository::get_all(&storage.conn).await
    }

    /// Retrieves all tasks for a specific project from local storage.
    ///
    /// # Arguments
    /// * `project_id` - The unique identifier of the project
    ///
    /// # Returns
    /// A vector of `task::Model` objects for the specified project
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_tasks_for_project(&self, project_id: &Uuid) -> Result<Vec<task::Model>> {
        let storage = self.storage.lock().await;
        TaskRepository::get_for_project(&storage.conn, project_id).await
    }

    /// Retrieves all tasks from local storage across all projects.
    ///
    /// This method is primarily used for search functionality and global task operations.
    /// It provides fast access to the complete task dataset.
    ///
    /// # Returns
    /// A vector of all `task::Model` objects in the local database
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_all_tasks(&self) -> Result<Vec<task::Model>> {
        let storage = self.storage.lock().await;
        TaskRepository::get_all(&storage.conn).await
    }

    /// Searches for tasks by content using database-level filtering.
    ///
    /// This method performs fast text search across task content using SQL LIKE queries.
    /// The search is case-insensitive and matches partial content.
    ///
    /// # Arguments
    /// * `query` - The search term to look for in task content
    ///
    /// # Returns
    /// A vector of `task::Model` objects matching the search criteria
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn search_tasks(&self, query: &str) -> Result<Vec<task::Model>> {
        let storage = self.storage.lock().await;
        TaskRepository::search(&storage.conn, query).await
    }

    /// Get all labels from local storage (fast)
    pub async fn get_labels(&self) -> Result<Vec<label::Model>> {
        let storage = self.storage.lock().await;
        LabelRepository::get_all(&storage.conn).await
    }

    /// Get all sections from local storage (fast)
    pub async fn get_sections(&self) -> Result<Vec<section::Model>> {
        let storage = self.storage.lock().await;
        SectionRepository::get_all(&storage.conn).await
    }

    /// Get sections for a project from local storage (fast)
    pub async fn get_sections_for_project(&self, project_uuid: &Uuid) -> Result<Vec<section::Model>> {
        let storage = self.storage.lock().await;
        SectionRepository::get_for_project(&storage.conn, project_uuid).await
    }

    /// Get tasks with a specific label from local storage (fast)
    pub async fn get_tasks_with_label(&self, label_id: Uuid) -> Result<Vec<task::Model>> {
        let storage = self.storage.lock().await;
        TaskRepository::get_with_label(&storage.conn, label_id).await
    }

    /// Retrieves tasks for the "Today" view with business logic.
    ///
    /// This method implements the UI business logic for the Today view by combining
    /// overdue tasks with tasks due today. Overdue tasks are shown first, followed
    /// by today's tasks.
    ///
    /// # Returns
    /// A vector of `task::Model` objects for the Today view, with overdue tasks first
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_tasks_for_today(&self) -> Result<Vec<task::Model>> {
        let storage = self.storage.lock().await;
        let today = datetime::format_today();
        TaskRepository::get_for_today(&storage.conn, &today).await
    }

    /// Retrieves tasks scheduled for tomorrow.
    ///
    /// This method returns only tasks that are specifically due tomorrow,
    /// without any additional business logic.
    ///
    /// # Returns
    /// A vector of `task::Model` objects due tomorrow
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_tasks_for_tomorrow(&self) -> Result<Vec<task::Model>> {
        let storage = self.storage.lock().await;
        let tomorrow = datetime::format_date_with_offset(1);
        TaskRepository::get_for_tomorrow(&storage.conn, &tomorrow).await
    }

    /// Retrieves tasks for the "Upcoming" view with business logic.
    ///
    /// This method implements the UI business logic for the Upcoming view by combining
    /// overdue tasks, today's tasks, and tasks due within the next 3 months.
    /// Tasks are ordered as: overdue → today → future (next 3 months).
    ///
    /// # Returns
    /// A vector of `task::Model` objects for the Upcoming view, properly ordered
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_tasks_for_upcoming(&self) -> Result<Vec<task::Model>> {
        let storage = self.storage.lock().await;
        let today = datetime::format_today();
        let three_months_later = datetime::format_date_with_offset(90);
        TaskRepository::get_for_upcoming(&storage.conn, &today, &three_months_later).await
    }

    /// Get a single task by ID from local storage (fast)
    pub async fn get_task_by_id(&self, task_id: &Uuid) -> Result<Option<task::Model>> {
        let storage = self.storage.lock().await;
        TaskRepository::get_by_id(&storage.conn, task_id).await
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

    /// Creates a new project via the remote backend and stores it locally.
    ///
    /// This method creates a project remotely and immediately stores it in local storage
    /// for instant UI updates. The project will be available in the UI without requiring
    /// a full sync operation.
    ///
    /// # Arguments
    /// * `name` - The name of the new project
    /// * `parent_uuid` - Optional parent project UUID for creating sub-projects
    ///
    /// # Note
    /// As of 2025, Todoist allows free plan users to create more than 5 projects via the backend,
    /// but the GET /projects backend endpoint will only return the first 5 projects for free users.
    ///
    /// # Errors
    /// Returns an error if the backend call fails or local storage update fails
    pub async fn create_project(&self, name: &str, parent_uuid: Option<Uuid>) -> Result<()> {
        // Look up remote_id for parent project if provided
        let remote_parent_id = if let Some(uuid) = parent_uuid {
            Some(self.get_project_remote_id(&uuid).await?)
        } else {
            None
        };

        // Create project via backend using backend CreateProjectArgs
        let project_args = crate::backend::CreateProjectArgs {
            name: name.to_string(),
            parent_remote_id: remote_parent_id,
            color: None,
            is_favorite: None,
        };
        let backend_project = self.get_backend().await?
            .create_project(project_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Store the created project in local database immediately for UI refresh
        let storage = self.storage.lock().await;

        // Upsert the project
        let local_project = project::ActiveModel {
            uuid: ActiveValue::Set(Uuid::new_v4()),
            backend_uuid: ActiveValue::Set(self.backend_uuid),
            remote_id: ActiveValue::Set(backend_project.remote_id),
            name: ActiveValue::Set(backend_project.name),
            color: ActiveValue::Set(backend_project.color),
            is_favorite: ActiveValue::Set(backend_project.is_favorite),
            is_inbox_project: ActiveValue::Set(backend_project.is_inbox),
            order_index: ActiveValue::Set(backend_project.order_index),
            parent_uuid: ActiveValue::Set(parent_uuid),
        };

        use sea_orm::sea_query::OnConflict;
        let mut insert = project::Entity::insert(local_project);
        insert = insert.on_conflict(
            OnConflict::columns([project::Column::BackendUuid, project::Column::RemoteId])
                .update_columns([
                    project::Column::Name,
                    project::Column::Color,
                    project::Column::IsFavorite,
                    project::Column::IsInboxProject,
                    project::Column::OrderIndex,
                ])
                .to_owned(),
        );
        insert.exec(&storage.conn).await?;

        Ok(())
    }

    /// Creates a new task via the remote backend and stores it locally.
    ///
    /// This method creates a task remotely and immediately stores it in local storage
    /// for instant UI updates. The task will be available in the UI without requiring
    /// a full sync operation.
    ///
    /// # Arguments
    /// * `content` - The content/description of the new task
    /// * `project_uuid` - Optional local project UUID to assign the task to a specific project
    ///
    /// # Errors
    /// Returns an error if the backend call fails or local storage update fails
    pub async fn create_task(&self, content: &str, project_uuid: Option<Uuid>) -> Result<()> {
        // Look up remote_id for project if provided
        let remote_project_id = if let Some(uuid) = project_uuid {
            Some(self.get_project_remote_id(&uuid).await?)
        } else {
            None
        };

        // Create task via backend using backend CreateTaskArgs
        let task_args = crate::backend::CreateTaskArgs {
            content: content.to_string(),
            description: None,
            project_remote_id: remote_project_id.unwrap_or_default(),
            section_remote_id: None,
            parent_remote_id: None,
            priority: None,
            due_date: None,
            due_datetime: None,
            duration: None,
            labels: Vec::new(),
        };
        let backend_task = self.get_backend().await?
            .create_task(task_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Store the created task in local database immediately for UI refresh
        let storage = self.storage.lock().await;
        let txn = storage.conn.begin().await?;

        // Look up local project UUID from remote project_id
        let project_uuid = Self::lookup_project_uuid(
            &txn,
            &self.backend_uuid,
            &backend_task.project_remote_id,
            "task creation",
        )
        .await?;

        // Look up local section UUID from remote section_id if present
        let section_uuid = Self::lookup_section_uuid(
            &txn,
            &self.backend_uuid,
            backend_task.section_remote_id.as_ref(),
        )
        .await?;

        // Look up local parent UUID from remote parent_id if present
        let parent_uuid = if let Some(remote_parent_id) = &backend_task.parent_remote_id {
            TaskRepository::get_by_remote_id(&txn, &self.backend_uuid, remote_parent_id)
                .await?
                .map(|t| t.uuid)
        } else {
            None
        };

        let local_task = task::ActiveModel {
            uuid: ActiveValue::Set(Uuid::new_v4()),
            backend_uuid: ActiveValue::Set(self.backend_uuid),
            remote_id: ActiveValue::Set(backend_task.remote_id),
            content: ActiveValue::Set(backend_task.content),
            description: ActiveValue::Set(backend_task.description),
            project_uuid: ActiveValue::Set(project_uuid),
            section_uuid: ActiveValue::Set(section_uuid),
            parent_uuid: ActiveValue::Set(parent_uuid),
            priority: ActiveValue::Set(backend_task.priority),
            order_index: ActiveValue::Set(backend_task.order_index),
            due_date: ActiveValue::Set(backend_task.due_date),
            due_datetime: ActiveValue::Set(backend_task.due_datetime),
            is_recurring: ActiveValue::Set(backend_task.is_recurring),
            deadline: ActiveValue::Set(backend_task.deadline),
            duration: ActiveValue::Set(backend_task.duration),
            is_completed: ActiveValue::Set(backend_task.is_completed),
            is_deleted: ActiveValue::Set(false),
        };

        use sea_orm::sea_query::OnConflict;
        let mut insert = task::Entity::insert(local_task);
        insert = insert.on_conflict(
            OnConflict::columns([task::Column::BackendUuid, task::Column::RemoteId])
                .update_columns([
                    task::Column::Content,
                    task::Column::Description,
                    task::Column::ProjectUuid,
                    task::Column::SectionUuid,
                    task::Column::ParentUuid,
                    task::Column::Priority,
                    task::Column::OrderIndex,
                    task::Column::DueDate,
                    task::Column::DueDatetime,
                    task::Column::IsRecurring,
                    task::Column::Deadline,
                    task::Column::Duration,
                    task::Column::IsCompleted,
                    task::Column::IsDeleted,
                ])
                .to_owned(),
        );
        insert.exec(&txn).await?;

        txn.commit().await?;

        Ok(())
    }

    /// Creates a new label via the remote backend and stores it locally.
    ///
    /// This method creates a label remotely and immediately stores it in local storage
    /// for instant UI updates. The label will be available in the UI without requiring
    /// a full sync operation.
    ///
    /// # Arguments
    /// * `name` - The name of the new label
    /// * `color` - Optional color for the label (hex code or predefined color name)
    ///
    /// # Errors
    /// Returns an error if the backend call fails or local storage update fails
    pub async fn create_label(&self, name: &str, color: Option<&str>) -> Result<()> {
        info!("Backend: Creating label '{}' with color {:?}", name, color);

        // Create label via backend using the CreateLabelArgs structure
        let label_args = crate::backend::CreateLabelArgs {
            name: name.to_string(),
            color: color.map(std::string::ToString::to_string),
            is_favorite: None,
        };
        let api_label = self.get_backend().await?
            .create_label(label_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Store the created label in local database immediately for UI refresh
        info!("Storage: Storing new label locally with ID {}", api_label.remote_id);
        let storage = self.storage.lock().await;

        let local_label = label::ActiveModel {
            uuid: ActiveValue::Set(Uuid::new_v4()),
            backend_uuid: ActiveValue::Set(self.backend_uuid),
            remote_id: ActiveValue::Set(api_label.remote_id),
            name: ActiveValue::Set(api_label.name),
            color: ActiveValue::Set(api_label.color),
            order_index: ActiveValue::Set(api_label.order_index),
            is_favorite: ActiveValue::Set(api_label.is_favorite),
        };

        use sea_orm::sea_query::OnConflict;
        let mut insert = label::Entity::insert(local_label);
        insert = insert.on_conflict(
            OnConflict::columns([label::Column::BackendUuid, label::Column::RemoteId])
                .update_columns([
                    label::Column::Name,
                    label::Column::Color,
                    label::Column::OrderIndex,
                    label::Column::IsFavorite,
                ])
                .to_owned(),
        );
        insert.exec(&storage.conn).await?;

        Ok(())
    }

    /// Update label content (name only for now)
    pub async fn update_label_content(&self, label_uuid: &Uuid, name: &str) -> Result<()> {
        info!("Backend: Updating label name for UUID {} to '{}'", label_uuid, name);

        // Look up the label's remote_id for backend call
        let remote_id = self.get_label_remote_id(label_uuid).await?;

        // Update label via backend using the UpdateLabelArgs structure
        let label_args = crate::backend::UpdateLabelArgs {
            name: Some(name.to_string()),
            color: None,
            is_favorite: None,
        };
        let _label = self.get_backend().await?
            .update_label(&remote_id, label_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Update local storage immediately after successful backend call
        info!(
            "Storage: Updating local label name for UUID {} to '{}'",
            label_uuid, name
        );
        let storage = self.storage.lock().await;

        if let Some(label) = LabelRepository::get_by_id(&storage.conn, label_uuid).await? {
            let mut active_model: label::ActiveModel = label.into_active_model();
            active_model.name = ActiveValue::Set(name.to_string());
            LabelRepository::update(&storage.conn, active_model).await?;
        }

        Ok(())
    }

    /// Delete a label
    pub async fn delete_label(&self, label_uuid: &Uuid) -> Result<()> {
        // Look up the label's remote_id for backend call
        let remote_id = self.get_label_remote_id(label_uuid).await?;

        // Delete label via backend
        self.get_backend()
            .await?
            .delete_label(&remote_id)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Note: Local storage deletion will be handled by the next sync
        Ok(())
    }

    /// Update project content (name only for now)
    pub async fn update_project_content(&self, project_uuid: &Uuid, name: &str) -> Result<()> {
        info!("Backend: Updating project name for UUID {} to '{}'", project_uuid, name);

        // Look up the project's remote_id for backend call
        let remote_id = self.get_project_remote_id(project_uuid).await?;

        // Update project via backend using the UpdateProjectArgs structure
        let project_args = crate::backend::UpdateProjectArgs {
            name: Some(name.to_string()),
            color: None,
            is_favorite: None,
        };
        let _project = self.get_backend().await?
            .update_project(&remote_id, project_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Update local storage immediately after successful backend call
        info!(
            "Storage: Updating local project name for UUID {} to '{}'",
            project_uuid, name
        );
        let storage = self.storage.lock().await;

        if let Some(project) = ProjectRepository::get_by_id(&storage.conn, project_uuid).await? {
            let mut active_model: project::ActiveModel = project.into_active_model();
            active_model.name = ActiveValue::Set(name.to_string());
            ProjectRepository::update(&storage.conn, active_model).await?;
        }

        Ok(())
    }

    /// Update task content
    pub async fn update_task_content(&self, task_uuid: &Uuid, content: &str) -> Result<()> {
        // Look up the task's remote_id for backend call
        let remote_id = self.get_task_remote_id(task_uuid).await?;

        // Update task via backend using the UpdateTaskArgs structure
        let task_args = crate::backend::UpdateTaskArgs {
            content: Some(content.to_string()),
            description: None,
            project_remote_id: None,
            section_remote_id: None,
            parent_remote_id: None,
            priority: None,
            due_date: None,
            due_datetime: None,
            duration: None,
            labels: None,
        };
        let _task = self.get_backend().await?
            .update_task(&remote_id, task_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Update local storage immediately after successful backend call
        info!(
            "Storage: Updating local task content for UUID {} to '{}'",
            task_uuid, content
        );
        let storage = self.storage.lock().await;

        if let Some(task) = TaskRepository::get_by_id(&storage.conn, task_uuid).await? {
            let mut active_model: task::ActiveModel = task.into_active_model();
            active_model.content = ActiveValue::Set(content.to_string());
            TaskRepository::update(&storage.conn, active_model).await?;
        }

        Ok(())
    }

    /// Update task due date
    pub async fn update_task_due_date(&self, task_uuid: &Uuid, due_date: Option<&str>) -> Result<()> {
        info!(
            "Backend: Updating task due date for UUID {} to {:?}",
            task_uuid, due_date
        );

        // Look up the task's remote_id for backend call
        let remote_id = self.get_task_remote_id(task_uuid).await?;

        // Update task via backend using the UpdateTaskArgs structure
        let task_args = crate::backend::UpdateTaskArgs {
            content: None,
            description: None,
            project_remote_id: None,
            section_remote_id: None,
            parent_remote_id: None,
            priority: None,
            due_date: due_date.map(std::string::ToString::to_string),
            due_datetime: None,
            duration: None,
            labels: None,
        };
        let _task = self.get_backend().await?
            .update_task(&remote_id, task_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Then update local storage
        let storage = self.storage.lock().await;

        if let Some(task) = TaskRepository::get_by_id(&storage.conn, task_uuid).await? {
            let mut active_model: task::ActiveModel = task.into_active_model();
            active_model.due_date = ActiveValue::Set(due_date.map(|s| s.to_string()));
            TaskRepository::update(&storage.conn, active_model).await?;
        }

        info!("Backend: Successfully updated task due date {}", task_uuid);
        Ok(())
    }

    /// Update task priority
    pub async fn update_task_priority(&self, task_uuid: &Uuid, priority: i32) -> Result<()> {
        info!("Backend: Updating task priority for UUID {} to {}", task_uuid, priority);

        // Look up the task's remote_id for backend call
        let remote_id = self.get_task_remote_id(task_uuid).await?;

        // Update task via backend using the UpdateTaskArgs structure
        let task_args = crate::backend::UpdateTaskArgs {
            content: None,
            description: None,
            project_remote_id: None,
            section_remote_id: None,
            parent_remote_id: None,
            priority: Some(priority),
            due_date: None,
            due_datetime: None,
            duration: None,
            labels: None,
        };
        let _task = self.get_backend().await?
            .update_task(&remote_id, task_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Then update local storage
        let storage = self.storage.lock().await;

        if let Some(task) = TaskRepository::get_by_id(&storage.conn, task_uuid).await? {
            let mut active_model: task::ActiveModel = task.into_active_model();
            active_model.priority = ActiveValue::Set(priority);
            TaskRepository::update(&storage.conn, active_model).await?;
        }

        info!("Backend: Successfully updated task priority {}", task_uuid);
        Ok(())
    }

    /// Delete a project
    pub async fn delete_project(&self, project_uuid: &Uuid) -> Result<()> {
        // Look up the project's remote_id for backend call
        let remote_id = self.get_project_remote_id(project_uuid).await?;

        // Delete project via backend
        self.get_backend()
            .await?
            .delete_project(&remote_id)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Remove from local storage
        let storage = self.storage.lock().await;

        if let Some(project) = ProjectRepository::get_by_id(&storage.conn, project_uuid).await? {
            ProjectRepository::delete(&storage.conn, project).await?;
        }

        Ok(())
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

        // Fetch projects from backend
        let projects = match self.get_backend().await?.fetch_projects().await {
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

        // Fetch all tasks from backend
        let tasks = match self.get_backend().await?.fetch_tasks().await {
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

        // Fetch all labels from backend
        let labels = match self.get_backend().await?.fetch_labels().await {
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

        // Fetch all sections from backend
        let sections = match self.get_backend().await?.fetch_sections().await {
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

            // Store projects
            if let Err(e) = self.store_projects_batch(&storage, &projects).await {
                error!("❌ Failed to store projects: {e}");
                return Ok(SyncStatus::Error {
                    message: format!("Failed to store projects: {e}"),
                });
            }
            info!("✅ Stored projects in database");

            // Store labels BEFORE tasks so task-label relationships can be created
            if let Err(e) = self.store_labels_batch(&storage, &labels).await {
                error!("❌ Failed to store labels: {e}");
                return Ok(SyncStatus::Error {
                    message: format!("Failed to store labels: {e}"),
                });
            }
            info!("✅ Stored labels in database");

            if let Err(e) = self.store_tasks_batch(&storage, &tasks).await {
                error!("❌ Failed to store tasks: {e}");
                return Ok(SyncStatus::Error {
                    message: format!("Failed to store tasks: {e}"),
                });
            }
            info!("✅ Stored tasks in database");

            if !sections.is_empty() {
                if let Err(e) = self.store_sections_batch(&storage, &sections).await {
                    error!("❌ Failed to store sections: {e}");
                    return Ok(SyncStatus::Error {
                        message: format!("Failed to store sections: {e}"),
                    });
                }
                info!("✅ Stored sections in database");
            } else {
                info!("⚠️  No sections to store (skipped due to backend issue)");
            }
        }

        Ok(SyncStatus::Success)
    }

    /// Look up remote_id from local task UUID (with automatic locking).
    ///
    /// # Arguments
    /// * `task_uuid` - Local task UUID
    ///
    /// # Returns
    /// Remote task ID for remote backend
    ///
    /// # Errors
    /// Returns error if task with given UUID doesn't exist locally
    async fn get_task_remote_id(&self, task_uuid: &Uuid) -> Result<String> {
        let storage = self.storage.lock().await;
        TaskRepository::get_remote_id(&storage.conn, task_uuid).await
    }

    /// Look up remote_id from local project UUID (with automatic locking).
    ///
    /// # Arguments
    /// * `project_uuid` - Local project UUID
    ///
    /// # Returns
    /// Remote project ID for remote backend
    ///
    /// # Errors
    /// Returns error if project with given UUID doesn't exist locally
    async fn get_project_remote_id(&self, project_uuid: &Uuid) -> Result<String> {
        let storage = self.storage.lock().await;
        ProjectRepository::get_remote_id(&storage.conn, project_uuid).await
    }

    /// Look up remote_id from local label UUID (with automatic locking).
    ///
    /// # Arguments
    /// * `label_uuid` - Local label UUID
    ///
    /// # Returns
    /// Remote label ID for remote backend
    ///
    /// # Errors
    /// Returns error if label with given UUID doesn't exist locally
    async fn get_label_remote_id(&self, label_uuid: &Uuid) -> Result<String> {
        let storage = self.storage.lock().await;
        LabelRepository::get_remote_id(&storage.conn, label_uuid).await
    }

    /// Look up local project UUID from remote project_id.
    ///
    /// # Arguments
    /// * `txn` - Database transaction
    /// * `remote_project_id` - Remote project ID from remote backend
    /// * `context` - Context string for error message (e.g., "task creation", "section sync")
    ///
    /// # Returns
    /// Local project UUID
    ///
    /// # Errors
    /// Returns error if project with given remote_id doesn't exist locally
    async fn lookup_project_uuid(
        txn: &sea_orm::DatabaseTransaction,
        backend_uuid: &Uuid,
        remote_project_id: &str,
        context: &str,
    ) -> Result<Uuid> {
        if let Some(project) = ProjectRepository::get_by_remote_id(txn, backend_uuid, remote_project_id).await? {
            Ok(project.uuid)
        } else {
            Err(anyhow::anyhow!(
                "Project with remote_id {} not found locally during {}. Please sync projects first.",
                remote_project_id,
                context
            ))
        }
    }

    /// Look up local section UUID from remote section_id.
    ///
    /// # Arguments
    /// * `txn` - Database transaction
    /// * `remote_section_id` - Remote section ID from remote backend
    ///
    /// # Returns
    /// Optional local section UUID (None if section_id is not provided)
    ///
    /// # Errors
    /// Returns error if database query fails
    async fn lookup_section_uuid(
        txn: &sea_orm::DatabaseTransaction,
        backend_uuid: &Uuid,
        remote_section_id: Option<&String>,
    ) -> Result<Option<Uuid>> {
        if let Some(remote_id) = remote_section_id {
            let section_uuid = SectionRepository::get_by_remote_id(txn, backend_uuid, remote_id)
                .await?
                .map(|s| s.uuid);
            Ok(section_uuid)
        } else {
            Ok(None)
        }
    }

    /// Store projects in batch
    async fn store_projects_batch(
        &self,
        storage: &LocalStorage,
        projects: &[crate::backend::BackendProject],
    ) -> Result<()> {
        use sea_orm::sea_query::OnConflict;

        let txn = storage.conn.begin().await?;

        // First pass: Upsert all projects without parent_uuid relationships
        for backend_project in projects {
            let local_project = project::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                backend_uuid: ActiveValue::Set(self.backend_uuid),
                remote_id: ActiveValue::Set(backend_project.remote_id.clone()),
                name: ActiveValue::Set(backend_project.name.clone()),
                color: ActiveValue::Set(backend_project.color.clone()),
                is_favorite: ActiveValue::Set(backend_project.is_favorite),
                is_inbox_project: ActiveValue::Set(backend_project.is_inbox),
                order_index: ActiveValue::Set(backend_project.order_index),
                parent_uuid: ActiveValue::Set(None),
            };

            let mut insert = project::Entity::insert(local_project);
            insert = insert.on_conflict(
                OnConflict::columns([project::Column::BackendUuid, project::Column::RemoteId])
                    .update_columns([
                        project::Column::Name,
                        project::Column::Color,
                        project::Column::IsFavorite,
                        project::Column::IsInboxProject,
                        project::Column::OrderIndex,
                        project::Column::ParentUuid,
                    ])
                    .to_owned(),
            );
            insert.exec(&txn).await?;
        }

        // Second pass: Update parent_uuid references to use local UUIDs
        for backend_project in projects {
            if let Some(remote_parent_id) = &backend_project.parent_remote_id {
                if let Some(parent) =
                    ProjectRepository::get_by_remote_id(&txn, &self.backend_uuid, remote_parent_id).await?
                {
                    if let Some(project) = ProjectRepository::get_by_remote_id(
                        &txn,
                        &self.backend_uuid,
                        &backend_project.remote_id,
                    )
                    .await?
                    {
                        let mut active_model: project::ActiveModel = project.into_active_model();
                        active_model.parent_uuid = ActiveValue::Set(Some(parent.uuid));
                        ProjectRepository::update(&txn, active_model).await?;
                    }
                }
            }
        }

        txn.commit().await?;
        Ok(())
    }

    /// Store labels in batch
    async fn store_labels_batch(&self, storage: &LocalStorage, labels: &[crate::backend::BackendLabel]) -> Result<()> {
        use sea_orm::sea_query::OnConflict;

        let txn = storage.conn.begin().await?;

        for backend_label in labels {
            let local_label = label::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                backend_uuid: ActiveValue::Set(self.backend_uuid),
                remote_id: ActiveValue::Set(backend_label.remote_id.clone()),
                name: ActiveValue::Set(backend_label.name.clone()),
                color: ActiveValue::Set(backend_label.color.clone()),
                order_index: ActiveValue::Set(backend_label.order_index),
                is_favorite: ActiveValue::Set(backend_label.is_favorite),
            };

            let mut insert = label::Entity::insert(local_label);
            insert = insert.on_conflict(
                OnConflict::columns([label::Column::BackendUuid, label::Column::RemoteId])
                    .update_columns([
                        label::Column::Name,
                        label::Column::Color,
                        label::Column::OrderIndex,
                        label::Column::IsFavorite,
                    ])
                    .to_owned(),
            );
            insert.exec(&txn).await?;
        }

        txn.commit().await?;
        Ok(())
    }

    /// Store tasks in batch
    async fn store_tasks_batch(&self, storage: &LocalStorage, tasks: &[crate::backend::BackendTask]) -> Result<()> {
        use sea_orm::sea_query::OnConflict;

        let txn = storage.conn.begin().await?;

        // Track task labels for later processing
        let mut task_labels_map: Vec<(Uuid, Vec<String>)> = Vec::new();

        // First pass: Upsert all tasks without parent_uuid relationships
        for backend_task in tasks {
            let label_names = backend_task.labels.clone();

            // Look up local project UUID from remote project_id
            let project_uuid = match Self::lookup_project_uuid(
                &txn,
                &self.backend_uuid,
                &backend_task.project_remote_id,
                "task batch sync",
            )
            .await
            {
                Ok(uuid) => uuid,
                Err(_) => {
                    // Skip tasks whose projects don't exist locally (can happen with free tier API limitations)
                    continue;
                }
            };

            // Look up local section UUID from remote section_id if present
            let section_uuid = Self::lookup_section_uuid(
                &txn,
                &self.backend_uuid,
                backend_task.section_remote_id.as_ref(),
            )
            .await?;

            let local_task = task::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                backend_uuid: ActiveValue::Set(self.backend_uuid),
                remote_id: ActiveValue::Set(backend_task.remote_id.clone()),
                content: ActiveValue::Set(backend_task.content.clone()),
                description: ActiveValue::Set(backend_task.description.clone()),
                project_uuid: ActiveValue::Set(project_uuid),
                section_uuid: ActiveValue::Set(section_uuid),
                parent_uuid: ActiveValue::Set(None),
                priority: ActiveValue::Set(backend_task.priority),
                order_index: ActiveValue::Set(backend_task.order_index),
                due_date: ActiveValue::Set(backend_task.due_date.clone()),
                due_datetime: ActiveValue::Set(backend_task.due_datetime.clone()),
                is_recurring: ActiveValue::Set(backend_task.is_recurring),
                deadline: ActiveValue::Set(backend_task.deadline.clone()),
                duration: ActiveValue::Set(backend_task.duration.clone()),
                is_completed: ActiveValue::Set(backend_task.is_completed),
                is_deleted: ActiveValue::Set(false),
            };

            let mut insert = task::Entity::insert(local_task);
            insert = insert.on_conflict(
                OnConflict::columns([task::Column::BackendUuid, task::Column::RemoteId])
                    .update_columns([
                        task::Column::Content,
                        task::Column::Description,
                        task::Column::ProjectUuid,
                        task::Column::SectionUuid,
                        task::Column::ParentUuid,
                        task::Column::Priority,
                        task::Column::OrderIndex,
                        task::Column::DueDate,
                        task::Column::DueDatetime,
                        task::Column::IsRecurring,
                        task::Column::Deadline,
                        task::Column::Duration,
                        task::Column::IsCompleted,
                        task::Column::IsDeleted,
                    ])
                    .to_owned(),
            );
            insert.exec(&txn).await?;

            // Get the uuid of the task we just inserted/updated
            if let Some(task) =
                TaskRepository::get_by_remote_id(&txn, &self.backend_uuid, &backend_task.remote_id).await?
            {
                task_labels_map.push((task.uuid, label_names));
            }
        }

        // Second pass: Update parent_uuid references to use local UUIDs
        for backend_task in tasks {
            if let Some(remote_parent_id) = &backend_task.parent_remote_id {
                if let Some(parent) =
                    TaskRepository::get_by_remote_id(&txn, &self.backend_uuid, remote_parent_id).await?
                {
                    if let Some(task) =
                        TaskRepository::get_by_remote_id(&txn, &self.backend_uuid, &backend_task.remote_id)
                            .await?
                    {
                        let mut active_model: task::ActiveModel = task.into_active_model();
                        active_model.parent_uuid = ActiveValue::Set(Some(parent.uuid));
                        TaskRepository::update(&txn, active_model).await?;
                    }
                }
            }
        }

        // Delete all existing task-label relationships
        task_label::Entity::delete_many().exec(&txn).await?;

        // Recreate relationships
        for (task_uuid, label_names) in task_labels_map {
            if !label_names.is_empty() {
                // Find label UUIDs by names
                for label_name in label_names {
                    if let Some(label) = LabelRepository::get_by_name(&txn, &label_name).await? {
                        let task_label_relation = task_label::ActiveModel {
                            task_uuid: ActiveValue::Set(task_uuid),
                            label_uuid: ActiveValue::Set(label.uuid),
                        };
                        task_label::Entity::insert(task_label_relation)
                            .on_conflict(
                                sea_orm::sea_query::OnConflict::columns([
                                    task_label::Column::TaskUuid,
                                    task_label::Column::LabelUuid,
                                ])
                                .do_nothing()
                                .to_owned(),
                            )
                            .exec(&txn)
                            .await?;
                    }
                }
            }
        }

        txn.commit().await?;
        Ok(())
    }

    /// Store sections in batch
    async fn store_sections_batch(
        &self,
        storage: &LocalStorage,
        sections: &[crate::backend::BackendSection],
    ) -> Result<()> {
        use sea_orm::sea_query::OnConflict;

        let txn = storage.conn.begin().await?;

        for backend_section in sections {
            // Look up local project UUID from remote project_id
            let project_uuid = Self::lookup_project_uuid(
                &txn,
                &self.backend_uuid,
                &backend_section.project_remote_id,
                "section sync",
            )
            .await?;

            let local_section = section::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                backend_uuid: ActiveValue::Set(self.backend_uuid),
                remote_id: ActiveValue::Set(backend_section.remote_id.clone()),
                name: ActiveValue::Set(backend_section.name.clone()),
                project_uuid: ActiveValue::Set(project_uuid),
                order_index: ActiveValue::Set(backend_section.order_index),
            };

            let mut insert = section::Entity::insert(local_section);
            insert = insert.on_conflict(
                OnConflict::columns([section::Column::BackendUuid, section::Column::RemoteId])
                    .update_columns([section::Column::Name, section::Column::ProjectUuid, section::Column::OrderIndex])
                    .to_owned(),
            );
            insert.exec(&txn).await?;
        }

        txn.commit().await?;
        Ok(())
    }

    /// Force sync regardless of last sync time
    pub async fn force_sync(&self) -> Result<SyncStatus> {
        self.sync().await
    }

    /// Marks a task as completed via the remote backend and removes it from local storage.
    ///
    /// This method completes the task remotely (which automatically handles subtasks)
    /// and removes it from local storage since completed tasks are not displayed in the UI.
    /// Subtasks are automatically deleted via database CASCADE constraints.
    ///
    /// # Arguments
    /// * `task_uuid` - The local UUID of the task to complete
    ///
    /// # Errors
    /// Returns an error if the backend call fails or local storage update fails
    pub async fn complete_task(&self, task_uuid: &Uuid) -> Result<()> {
        // Look up the task's remote_id for backend call
        let remote_id = self.get_task_remote_id(task_uuid).await?;

        // Complete the task via backend using remote_id (this handles subtasks automatically)
        self.get_backend()
            .await?
            .complete_task(&remote_id)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Then mark as completed in local storage (soft completion)
        let storage = self.storage.lock().await;

        if let Some(task) = TaskRepository::get_by_id(&storage.conn, task_uuid).await? {
            let mut active_model: task::ActiveModel = task.into_active_model();
            active_model.is_completed = ActiveValue::Set(true);
            TaskRepository::update(&storage.conn, active_model).await?;
        }

        Ok(())
    }

    /// Permanently deletes a task via the remote backend and removes it from local storage.
    ///
    /// This method performs a hard delete of the task remotely, soft delete locally.
    /// The task will be permanently removed and cannot be recovered.
    ///
    /// # Arguments
    /// * `task_uuid` - The local UUID of the task to delete
    ///
    /// # Errors
    /// Returns an error if the backend call fails or local storage update fails
    pub async fn delete_task(&self, task_uuid: &Uuid) -> Result<()> {
        // Look up the task's remote_id for backend call
        let remote_id = self.get_task_remote_id(task_uuid).await?;

        // Delete the task via backend using remote_id
        self.get_backend()
            .await?
            .delete_task(&remote_id)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Then mark as deleted in local storage (soft deletion)
        let storage = self.storage.lock().await;

        if let Some(task) = TaskRepository::get_by_id(&storage.conn, task_uuid).await? {
            let mut active_model: task::ActiveModel = task.into_active_model();
            active_model.is_deleted = ActiveValue::Set(true);
            TaskRepository::update(&storage.conn, active_model).await?;
        }

        Ok(())
    }

    /// Restore a soft-deleted or completed task via the remote backend and locally
    /// For completed tasks, reopens them. For deleted tasks, recreates them via backend.
    pub async fn restore_task(&self, task_id: &Uuid) -> Result<()> {
        // First, get the task from local storage to check its state
        let storage = self.storage.lock().await;
        let task = TaskRepository::get_by_id(&storage.conn, task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found in local storage: {}", task_id))?;

        if task.is_deleted {
            // For deleted tasks, we need to recreate them via backend
            // Look up remote IDs before dropping storage lock
            let remote_project_id = ProjectRepository::get_remote_id(&storage.conn, &task.project_uuid).await?;
            let remote_section_id = if let Some(section_uuid) = &task.section_uuid {
                SectionRepository::get_remote_id(&storage.conn, section_uuid).await?
            } else {
                None
            };
            let remote_parent_id = if let Some(parent_uuid) = &task.parent_uuid {
                Some(TaskRepository::get_remote_id(&storage.conn, parent_uuid).await?)
            } else {
                None
            };

            drop(storage); // Release the lock before API call

            // Create the task again via backend
            let task_args = crate::backend::CreateTaskArgs {
                content: task.content.clone(),
                description: task.description.clone().filter(|d| !d.is_empty()),
                project_remote_id: remote_project_id,
                section_remote_id: remote_section_id,
                parent_remote_id: remote_parent_id,
                priority: Some(task.priority),
                due_date: task.due_date.clone(),
                due_datetime: task.due_datetime.clone(),
                duration: task.duration.clone(),
                labels: Vec::new(), // Labels will be synced separately
            };

            let new_task = self.get_backend().await?
                .create_task(task_args)
                .await
                .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

            // Update local storage: remove the old soft-deleted task and add the new one
            let storage = self.storage.lock().await;

            // Hard delete the old soft-deleted task
            if let Some(old_task) = TaskRepository::get_by_id(&storage.conn, task_id).await? {
                TaskRepository::delete(&storage.conn, old_task).await?;
            }

            // Store the new task (reuse the single task upsert logic)
            let txn = storage.conn.begin().await?;

            let project_uuid = Self::lookup_project_uuid(
                &txn,
                &self.backend_uuid,
                &new_task.project_remote_id,
                "task restore",
            )
            .await?;

            let section_uuid =
                Self::lookup_section_uuid(&txn, &self.backend_uuid, new_task.section_remote_id.as_ref())
                    .await?;

            let parent_uuid = if let Some(remote_parent_id) = &new_task.parent_remote_id {
                TaskRepository::get_by_remote_id(&txn, &self.backend_uuid, remote_parent_id)
                    .await?
                    .map(|t| t.uuid)
            } else {
                None
            };

            let local_task = task::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                backend_uuid: ActiveValue::Set(self.backend_uuid),
                remote_id: ActiveValue::Set(new_task.remote_id),
                content: ActiveValue::Set(new_task.content),
                description: ActiveValue::Set(new_task.description),
                project_uuid: ActiveValue::Set(project_uuid),
                section_uuid: ActiveValue::Set(section_uuid),
                parent_uuid: ActiveValue::Set(parent_uuid),
                priority: ActiveValue::Set(new_task.priority),
                order_index: ActiveValue::Set(new_task.order_index),
                due_date: ActiveValue::Set(new_task.due_date),
                due_datetime: ActiveValue::Set(new_task.due_datetime),
                is_recurring: ActiveValue::Set(new_task.is_recurring),
                deadline: ActiveValue::Set(new_task.deadline),
                duration: ActiveValue::Set(new_task.duration),
                is_completed: ActiveValue::Set(new_task.is_completed),
                is_deleted: ActiveValue::Set(false),
            };

            use sea_orm::sea_query::OnConflict;
            let mut insert = task::Entity::insert(local_task);
            insert = insert.on_conflict(
                OnConflict::columns([task::Column::BackendUuid, task::Column::RemoteId])
                    .update_columns([
                        task::Column::Content,
                        task::Column::Description,
                        task::Column::ProjectUuid,
                        task::Column::SectionUuid,
                        task::Column::ParentUuid,
                        task::Column::Priority,
                        task::Column::OrderIndex,
                        task::Column::DueDate,
                        task::Column::DueDatetime,
                        task::Column::IsRecurring,
                        task::Column::Deadline,
                        task::Column::Duration,
                        task::Column::IsCompleted,
                        task::Column::IsDeleted,
                    ])
                    .to_owned(),
            );
            insert.exec(&txn).await?;

            txn.commit().await?;
        } else {
            // For completed tasks, just reopen them
            let remote_id = task.remote_id.clone();
            drop(storage); // Release the lock before API call
            self.get_backend()
                .await?
                .reopen_task(&remote_id)
                .await
                .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

            // Clear local completion flag
            let storage = self.storage.lock().await;

            if let Some(task) = TaskRepository::get_by_id(&storage.conn, task_id).await? {
                let mut active_model: task::ActiveModel = task.into_active_model();
                active_model.is_completed = ActiveValue::Set(false);
                TaskRepository::update(&storage.conn, active_model).await?;
            }
        }

        Ok(())
    }
}
