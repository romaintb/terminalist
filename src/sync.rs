//! Synchronization service module for the terminalist application.
//!
//! This module provides the [`SyncService`] struct which handles data synchronization
//! between the Todoist API and local storage. It manages tasks, projects, labels, and sections,
//! providing both read and write operations with proper error handling and logging.
//!
//! The sync service acts as the main data layer for the application, offering:
//! - Fast local data access for UI operations
//! - Background synchronization with Todoist API
//! - CRUD operations for tasks, projects, and labels
//! - Business logic for special views (Today, Tomorrow, Upcoming)

use anyhow::Result;
use log::{error, info};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, IntoActiveModel, ModelTrait, QueryFilter, QueryOrder,
    TransactionTrait,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::entities::{label, project, section, task, task_label};
use crate::storage::LocalStorage;
use crate::todoist::{CreateProjectArgs, TodoistWrapper};
use crate::utils::datetime;

/// Service that manages data synchronization between the Todoist API and local storage.
///
/// The `SyncService` acts as the primary data access layer for the application,
/// providing both fast local data retrieval and background synchronization with
/// the Todoist API. It handles all CRUD operations for tasks, projects, labels,
/// and sections while maintaining data consistency between local and remote storage.
///
/// # Features
/// - Thread-safe operations using Arc<Mutex<>>
/// - Prevents concurrent sync operations
/// - Provides immediate UI updates after create/update operations
/// - Handles business logic for special views (Today, Tomorrow, Upcoming)
/// - Optional logging support for debugging and monitoring
///
/// # Example
/// ```rust,no_run
/// use terminalist::sync::SyncService;
///
/// # async fn example() -> anyhow::Result<()> {
/// let sync_service = SyncService::new("api_token".to_string(), false,).await?;
///
/// // Sync data from Todoist API
/// sync_service.sync().await?;
///
/// // Get projects from local storage (fast)
/// let projects = sync_service.get_projects().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct SyncService {
    todoist: TodoistWrapper,
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
    /// Creates a new `SyncService` instance with the provided configuration.
    ///
    /// This initializes the Todoist API wrapper, local storage, and optional logging
    /// based on the provided configuration. The service is ready to perform sync
    /// operations immediately after creation.
    ///
    /// # Arguments
    /// * `api_token` - The Todoist API token for authentication
    /// * `debug_mode` - Whether to enable debug mode for local storage
    ///
    /// # Returns
    /// A new `SyncService` instance ready for use
    ///
    /// # Errors
    /// Returns an error if local storage initialization fails
    pub async fn new(api_token: String, debug_mode: bool) -> Result<Self> {
        let todoist = TodoistWrapper::new(api_token);
        let storage = Arc::new(Mutex::new(LocalStorage::new(debug_mode).await?));
        let sync_in_progress = Arc::new(Mutex::new(false));

        Ok(Self {
            todoist,
            storage,
            sync_in_progress,
            debug_mode,
        })
    }

    /// Returns whether debug mode is enabled.
    ///
    /// This is used to enable debug-only features like local data refresh.
    pub fn is_debug_mode(&self) -> bool {
        self.debug_mode
    }

    /// Retrieves all projects from local storage.
    ///
    /// This method provides fast access to cached project data without making API calls.
    /// Projects are sorted and ready for display in the UI.
    ///
    /// # Returns
    /// A vector of `project::Model` objects representing all available projects
    ///
    /// # Note
    /// As of 2025, Todoist allows free plan users to create more than 5 projects via the API,
    /// but the GET /projects API endpoint will only return the first 5 projects for free users.
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_projects(&self) -> Result<Vec<project::Model>> {
        let storage = self.storage.lock().await;
        let projects = project::Entity::find()
            .order_by_asc(project::Column::OrderIndex)
            .all(&storage.conn)
            .await?;
        Ok(projects)
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
        let tasks = task::Entity::find()
            .filter(task::Column::ProjectUuid.eq(*project_id))
            .order_by_asc(task::Column::IsDeleted)    // Deleted (true) last
            .order_by_asc(task::Column::IsCompleted)  // Within non-deleted: active (false) first, completed (true) second
            .order_by_asc(task::Column::OrderIndex)
            .all(&storage.conn)
            .await?;
        Ok(tasks)
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
        let tasks = task::Entity::find()
            .order_by_asc(task::Column::IsDeleted)  // Non-deleted (false) first, deleted (true) last
            .order_by_asc(task::Column::OrderIndex)
            .all(&storage.conn)
            .await?;
        Ok(tasks)
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
        use sea_orm::sea_query::Expr;
        let storage = self.storage.lock().await;
        let tasks = task::Entity::find()
            .filter(
                Expr::col(task::Column::Content)
                    .like(format!("%{}%", query))
                    .or(Expr::col(task::Column::Description).like(format!("%{}%", query))),
            )
            .order_by_asc(task::Column::IsDeleted)    // Deleted (true) last
            .order_by_asc(task::Column::IsCompleted)  // Within non-deleted: active (false) first, completed (true) second
            .order_by_asc(task::Column::OrderIndex)
            .all(&storage.conn)
            .await?;
        Ok(tasks)
    }

    /// Get all labels from local storage (fast)
    pub async fn get_labels(&self) -> Result<Vec<label::Model>> {
        let storage = self.storage.lock().await;
        let labels = label::Entity::find()
            .order_by_asc(label::Column::OrderIndex)
            .all(&storage.conn)
            .await?;
        Ok(labels)
    }

    /// Get all sections from local storage (fast)
    pub async fn get_sections(&self) -> Result<Vec<section::Model>> {
        let storage = self.storage.lock().await;
        let sections = section::Entity::find()
            .order_by_asc(section::Column::OrderIndex)
            .all(&storage.conn)
            .await?;
        Ok(sections)
    }

    /// Get sections for a project from local storage (fast)
    pub async fn get_sections_for_project(&self, project_id: &str) -> Result<Vec<section::Model>> {
        let storage = self.storage.lock().await;
        let sections = section::Entity::find()
            .filter(section::Column::ProjectUuid.eq(project_id))
            .order_by_asc(section::Column::OrderIndex)
            .all(&storage.conn)
            .await?;
        Ok(sections)
    }

    /// Get tasks with a specific label from local storage (fast)
    pub async fn get_tasks_with_label(&self, label_name: &str) -> Result<Vec<task::Model>> {
        // TODO: Implement label filtering - requires fetching labels per task
        // For now, return empty list as a workaround
        let _ = label_name;
        Ok(Vec::new())
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

        // Get overdue tasks (deleted last, then active → completed within non-deleted)
        let overdue_tasks = task::Entity::find()
            .filter(task::Column::DueDate.is_not_null())
            .filter(task::Column::DueDate.lt(&today))
            .order_by_asc(task::Column::IsDeleted)    // Deleted last
            .order_by_asc(task::Column::IsCompleted)  // Within non-deleted: active first, completed second
            .order_by_asc(task::Column::DueDate)
            .all(&storage.conn)
            .await?;

        // Get today's tasks (deleted last, then active → completed within non-deleted)
        let today_tasks = task::Entity::find()
            .filter(task::Column::DueDate.eq(&today))
            .order_by_asc(task::Column::IsDeleted)    // Deleted last
            .order_by_asc(task::Column::IsCompleted)  // Within non-deleted: active first, completed second
            .order_by_asc(task::Column::OrderIndex)
            .all(&storage.conn)
            .await?;

        // UI business rule: show overdue first, then today
        let mut result = overdue_tasks;
        result.extend(today_tasks);
        Ok(result)
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

        let tasks = task::Entity::find()
            .filter(task::Column::DueDate.eq(&tomorrow))
            .order_by_asc(task::Column::IsDeleted)    // Deleted (true) last
            .order_by_asc(task::Column::IsCompleted)  // Within non-deleted: active (false) first, completed (true) second
            .order_by_asc(task::Column::OrderIndex)
            .all(&storage.conn)
            .await?;
        Ok(tasks)
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

        // Get overdue tasks (deleted last, then active → completed within non-deleted)
        let overdue_tasks = task::Entity::find()
            .filter(task::Column::DueDate.is_not_null())
            .filter(task::Column::DueDate.lt(&today))
            .order_by_asc(task::Column::IsDeleted)    // Deleted last
            .order_by_asc(task::Column::IsCompleted)  // Within non-deleted: active first, completed second
            .order_by_asc(task::Column::DueDate)
            .all(&storage.conn)
            .await?;

        // Get today's tasks (deleted last, then active → completed within non-deleted)
        let today_tasks = task::Entity::find()
            .filter(task::Column::DueDate.eq(&today))
            .order_by_asc(task::Column::IsDeleted)    // Deleted last
            .order_by_asc(task::Column::IsCompleted)  // Within non-deleted: active first, completed second
            .order_by_asc(task::Column::OrderIndex)
            .all(&storage.conn)
            .await?;

        // Get future tasks (within next 3 months, deleted last, then active → completed)
        let future_tasks = task::Entity::find()
            .filter(task::Column::DueDate.gte(&today))
            .filter(task::Column::DueDate.lt(&three_months_later))
            .order_by_asc(task::Column::IsDeleted)    // Deleted last
            .order_by_asc(task::Column::IsCompleted)  // Within non-deleted: active first, completed second
            .order_by_asc(task::Column::DueDate)
            .all(&storage.conn)
            .await?;

        // UI business rule: overdue → today → future
        let mut result = overdue_tasks;
        result.extend(today_tasks);
        result.extend(future_tasks.into_iter().filter(|task| {
            // Remove today tasks from future to avoid duplicates
            if let Some(due_date) = &task.due_date {
                due_date != &today
            } else {
                true
            }
        }));
        Ok(result)
    }

    /// Get a single task by ID from local storage (fast)
    pub async fn get_task_by_id(&self, task_id: &Uuid) -> Result<Option<task::Model>> {
        let storage = self.storage.lock().await;
        let task = task::Entity::find()
            .filter(task::Column::Uuid.eq(*task_id))
            .one(&storage.conn)
            .await?;
        Ok(task)
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

    /// Creates a new project via the Todoist API and stores it locally.
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
    /// As of 2025, Todoist allows free plan users to create more than 5 projects via the API,
    /// but the GET /projects API endpoint will only return the first 5 projects for free users.
    ///
    /// # Errors
    /// Returns an error if the API call fails or local storage update fails
    pub async fn create_project(&self, name: &str, parent_uuid: Option<Uuid>) -> Result<()> {
        // Look up remote_id for parent project if provided
        let remote_parent_id = if let Some(uuid) = parent_uuid {
            let storage = self.storage.lock().await;
            let remote_id = Self::lookup_project_remote_id(&storage.conn, &uuid).await?;
            drop(storage);
            Some(remote_id)
        } else {
            None
        };

        // Create project via API using the new CreateProjectArgs structure
        let project_args = CreateProjectArgs {
            name: name.to_string(),
            color: None,
            parent_id: remote_parent_id,
            is_favorite: None,
            view_style: None,
        };
        let api_project = self.todoist.create_project(&project_args).await?;

        // Store the created project in local database immediately for UI refresh
        let storage = self.storage.lock().await;

        // Upsert the project
        let local_project = project::ActiveModel {
            uuid: ActiveValue::Set(Uuid::new_v4()),
            remote_id: ActiveValue::Set(api_project.id),
            name: ActiveValue::Set(api_project.name),
            color: ActiveValue::Set(api_project.color),
            is_favorite: ActiveValue::Set(api_project.is_favorite),
            is_inbox_project: ActiveValue::Set(api_project.is_inbox_project),
            order_index: ActiveValue::Set(api_project.order),
            parent_uuid: ActiveValue::Set(parent_uuid),
        };

        use sea_orm::sea_query::OnConflict;
        let mut insert = project::Entity::insert(local_project);
        insert = insert.on_conflict(
            OnConflict::column(project::Column::RemoteId)
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

    /// Creates a new task via the Todoist API and stores it locally.
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
    /// Returns an error if the API call fails or local storage update fails
    pub async fn create_task(&self, content: &str, project_uuid: Option<Uuid>) -> Result<()> {
        // Look up remote_id for project if provided
        let remote_project_id = if let Some(uuid) = project_uuid {
            let storage = self.storage.lock().await;
            let remote_id = Self::lookup_project_remote_id(&storage.conn, &uuid).await?;
            drop(storage);
            Some(remote_id)
        } else {
            None
        };

        // Create task via API using the new CreateTaskArgs structure
        let task_args = todoist_api::CreateTaskArgs {
            content: content.to_string(),
            description: None,
            project_id: remote_project_id,
            section_id: None,
            parent_id: None,
            order: None,
            priority: None,
            labels: None,
            due_string: None,
            due_date: None,
            due_datetime: None,
            due_lang: None,
            deadline_date: None,
            deadline_lang: None,
            assignee_id: None,
            duration: None,
            duration_unit: None,
        };
        let api_task = self.todoist.create_task(&task_args).await?;

        // Store the created task in local database immediately for UI refresh
        let storage = self.storage.lock().await;
        let txn = storage.conn.begin().await?;

        // Look up local project UUID from remote project_id
        let project_uuid = Self::lookup_project_uuid(&txn, &api_task.project_id, "task creation").await?;

        // Look up local section UUID from remote section_id if present
        let section_uuid = Self::lookup_section_uuid(&txn, api_task.section_id.as_ref()).await?;

        // Look up local parent UUID from remote parent_id if present
        let parent_uuid = if let Some(remote_parent_id) = &api_task.parent_id {
            task::Entity::find()
                .filter(task::Column::RemoteId.eq(remote_parent_id))
                .one(&txn)
                .await?
                .map(|t| t.uuid)
        } else {
            None
        };

        let duration_string = api_task.duration.as_ref().map(|d| match d.unit.as_str() {
            "minute" => format!("{}m", d.amount),
            "hour" => format!("{}h", d.amount),
            "day" => format!("{}d", d.amount),
            _ => format!("{} {}", d.amount, d.unit),
        });

        let local_task = task::ActiveModel {
            uuid: ActiveValue::Set(Uuid::new_v4()),
            remote_id: ActiveValue::Set(api_task.id),
            content: ActiveValue::Set(api_task.content),
            description: ActiveValue::Set(Some(api_task.description)),
            project_uuid: ActiveValue::Set(project_uuid),
            section_uuid: ActiveValue::Set(section_uuid),
            parent_uuid: ActiveValue::Set(parent_uuid),
            priority: ActiveValue::Set(api_task.priority),
            order_index: ActiveValue::Set(api_task.order),
            due_date: ActiveValue::Set(api_task.due.as_ref().map(|d| d.date.clone())),
            due_datetime: ActiveValue::Set(api_task.due.as_ref().and_then(|d| d.datetime.clone())),
            is_recurring: ActiveValue::Set(api_task.due.as_ref().is_some_and(|d| d.is_recurring)),
            deadline: ActiveValue::Set(api_task.deadline.as_ref().map(|d| d.date.clone())),
            duration: ActiveValue::Set(duration_string),
            is_completed: ActiveValue::Set(api_task.is_completed),
            is_deleted: ActiveValue::Set(false),
        };

        use sea_orm::sea_query::OnConflict;
        let mut insert = task::Entity::insert(local_task);
        insert = insert.on_conflict(
            OnConflict::column(task::Column::RemoteId)
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

    /// Creates a new label via the Todoist API and stores it locally.
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
    /// Returns an error if the API call fails or local storage update fails
    pub async fn create_label(&self, name: &str, color: Option<&str>) -> Result<()> {
        info!("API: Creating label '{}' with color {:?}", name, color);

        // Create label via API using the CreateLabelArgs structure
        let label_args = todoist_api::CreateLabelArgs {
            name: name.to_string(),
            color: color.map(std::string::ToString::to_string),
            order: None,
            is_favorite: None,
        };
        let api_label = self.todoist.create_label(&label_args).await?;

        // Store the created label in local database immediately for UI refresh
        info!("Storage: Storing new label locally with ID {}", api_label.id);
        let storage = self.storage.lock().await;

        let local_label = label::ActiveModel {
            uuid: ActiveValue::Set(Uuid::new_v4()),
            remote_id: ActiveValue::Set(api_label.id),
            name: ActiveValue::Set(api_label.name),
            color: ActiveValue::Set(api_label.color),
            order_index: ActiveValue::Set(api_label.order),
            is_favorite: ActiveValue::Set(api_label.is_favorite),
        };

        use sea_orm::sea_query::OnConflict;
        let mut insert = label::Entity::insert(local_label);
        insert = insert.on_conflict(
            OnConflict::column(label::Column::RemoteId)
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
        info!("API: Updating label name for UUID {} to '{}'", label_uuid, name);

        // Look up the label's remote_id for API call
        let storage = self.storage.lock().await;
        let remote_id = Self::lookup_label_remote_id(&storage.conn, label_uuid).await?;
        drop(storage);

        // Update label via API using the UpdateLabelArgs structure
        let label_args = todoist_api::UpdateLabelArgs {
            name: Some(name.to_string()),
            // Set all other fields to None to avoid overwriting existing data
            color: None,
            order: None,
            is_favorite: None,
        };
        let _label = self.todoist.update_label(&remote_id, &label_args).await?;

        // Update local storage immediately after successful API call
        info!("Storage: Updating local label name for UUID {} to '{}'", label_uuid, name);
        let storage = self.storage.lock().await;

        let label = label::Entity::find()
            .filter(label::Column::Uuid.eq(*label_uuid))
            .one(&storage.conn)
            .await?;

        if let Some(label) = label {
            let mut active_model: label::ActiveModel = label.into_active_model();
            active_model.name = ActiveValue::Set(name.to_string());
            active_model.update(&storage.conn).await?;
        }

        Ok(())
    }

    /// Delete a label
    pub async fn delete_label(&self, label_uuid: &Uuid) -> Result<()> {
        // Look up the label's remote_id for API call
        let storage = self.storage.lock().await;
        let remote_id = Self::lookup_label_remote_id(&storage.conn, label_uuid).await?;
        drop(storage);

        // Delete label via API
        self.todoist.delete_label(&remote_id).await?;

        // Note: Local storage deletion will be handled by the next sync
        Ok(())
    }

    /// Update project content (name only for now)
    pub async fn update_project_content(&self, project_uuid: &Uuid, name: &str) -> Result<()> {
        info!("API: Updating project name for UUID {} to '{}'", project_uuid, name);

        // Look up the project's remote_id for API call
        let storage = self.storage.lock().await;
        let remote_id = Self::lookup_project_remote_id(&storage.conn, project_uuid).await?;
        drop(storage);

        // Update project via API using the UpdateProjectArgs structure
        let project_args = todoist_api::UpdateProjectArgs {
            name: Some(name.to_string()),
            // Set all other fields to None to avoid overwriting existing data
            color: None,
            is_favorite: None,
            view_style: None,
        };
        let _project = self.todoist.update_project(&remote_id, &project_args).await?;

        // Update local storage immediately after successful API call
        info!(
            "Storage: Updating local project name for UUID {} to '{}'",
            project_uuid, name
        );
        let storage = self.storage.lock().await;

        let project = project::Entity::find()
            .filter(project::Column::Uuid.eq(*project_uuid))
            .one(&storage.conn)
            .await?;

        if let Some(project) = project {
            let mut active_model: project::ActiveModel = project.into_active_model();
            active_model.name = ActiveValue::Set(name.to_string());
            active_model.update(&storage.conn).await?;
        }

        Ok(())
    }

    /// Update task content
    pub async fn update_task_content(&self, task_uuid: &Uuid, content: &str) -> Result<()> {
        // Look up the task's remote_id for API call
        let storage = self.storage.lock().await;
        let remote_id = Self::lookup_task_remote_id(&storage.conn, task_uuid).await?;
        drop(storage);

        // Update task via API using the UpdateTaskArgs structure
        let task_args = todoist_api::UpdateTaskArgs {
            content: Some(content.to_string()),
            description: None,
            labels: None,
            priority: None,
            due_string: None,
            due_date: None,
            due_datetime: None,
            due_lang: None,
            deadline_date: None,
            deadline_lang: None,
            assignee_id: None,
            duration: None,
            duration_unit: None,
        };
        let _task = self.todoist.update_task(&remote_id, &task_args).await?;

        // The UI will handle the sync separately to ensure proper error handling
        Ok(())
    }

    /// Update task due date
    pub async fn update_task_due_date(&self, task_uuid: &Uuid, due_date: Option<&str>) -> Result<()> {
        info!("API: Updating task due date for UUID {} to {:?}", task_uuid, due_date);

        // Look up the task's remote_id for API call
        let storage = self.storage.lock().await;
        let remote_id = Self::lookup_task_remote_id(&storage.conn, task_uuid).await?;
        drop(storage);

        // Update task via API using the UpdateTaskArgs structure
        let task_args = todoist_api::UpdateTaskArgs {
            content: None,
            description: None,
            labels: None,
            priority: None,
            due_string: None,
            due_date: due_date.map(std::string::ToString::to_string),
            due_datetime: None,
            due_lang: None,
            deadline_date: None,
            deadline_lang: None,
            assignee_id: None,
            duration: None,
            duration_unit: None,
        };
        let _task = self.todoist.update_task(&remote_id, &task_args).await?;

        // Then update local storage
        let storage = self.storage.lock().await;

        let task = task::Entity::find()
            .filter(task::Column::Uuid.eq(*task_uuid))
            .one(&storage.conn)
            .await?;

        if let Some(task) = task {
            let mut active_model: task::ActiveModel = task.into_active_model();
            active_model.due_date = ActiveValue::Set(due_date.map(|s| s.to_string()));
            active_model.update(&storage.conn).await?;
        }

        info!("API: Successfully updated task due date {}", task_uuid);
        Ok(())
    }

    /// Update task priority
    pub async fn update_task_priority(&self, task_uuid: &Uuid, priority: i32) -> Result<()> {
        info!("API: Updating task priority for UUID {} to {}", task_uuid, priority);

        // Look up the task's remote_id for API call
        let storage = self.storage.lock().await;
        let remote_id = Self::lookup_task_remote_id(&storage.conn, task_uuid).await?;
        drop(storage);

        // Update task via API using the UpdateTaskArgs structure
        let task_args = todoist_api::UpdateTaskArgs {
            content: None,
            description: None,
            labels: None,
            priority: Some(priority),
            due_string: None,
            due_date: None,
            due_datetime: None,
            due_lang: None,
            deadline_date: None,
            deadline_lang: None,
            assignee_id: None,
            duration: None,
            duration_unit: None,
        };
        let _task = self.todoist.update_task(&remote_id, &task_args).await?;

        // Then update local storage
        let storage = self.storage.lock().await;

        let task = task::Entity::find()
            .filter(task::Column::Uuid.eq(*task_uuid))
            .one(&storage.conn)
            .await?;

        if let Some(task) = task {
            let mut active_model: task::ActiveModel = task.into_active_model();
            active_model.priority = ActiveValue::Set(priority);
            active_model.update(&storage.conn).await?;
        }

        info!("API: Successfully updated task priority {}", task_uuid);
        Ok(())
    }

    /// Delete a project
    pub async fn delete_project(&self, project_uuid: &Uuid) -> Result<()> {
        // Look up the project's remote_id for API call
        let storage = self.storage.lock().await;
        let remote_id = Self::lookup_project_remote_id(&storage.conn, project_uuid).await?;
        drop(storage);

        // Delete project via API
        self.todoist.delete_project(&remote_id).await?;

        // Remove from local storage
        let storage = self.storage.lock().await;

        if let Some(project) = project::Entity::find()
            .filter(project::Column::Uuid.eq(*project_uuid))
            .one(&storage.conn)
            .await?
        {
            project.delete(&storage.conn).await?;
        }

        Ok(())
    }

    /// Performs a full synchronization with the Todoist API.
    ///
    /// This method fetches all projects, tasks, labels, and sections from the Todoist API
    /// and stores them in local storage. It ensures that only one sync operation can run
    /// at a time to prevent data corruption and resource conflicts.
    ///
    /// The sync process includes:
    /// 1. Fetching projects, tasks, labels, and sections from the API
    /// 2. Storing all data in local storage with proper ordering
    /// 3. Handling API errors gracefully with detailed error messages
    /// 4. Providing progress logging if a logger is configured
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

        // Fetch projects from API
        let projects = match self.todoist.get_projects().await {
            Ok(projects) => {
                info!("✅ Fetched {} projects from API", projects.len());
                projects
            }
            Err(e) => {
                error!("❌ Failed to fetch projects: {e}");
                return Ok(SyncStatus::Error {
                    message: format!("Failed to fetch projects: {e}"),
                });
            }
        };

        // Fetch all tasks from API
        let tasks = match self.todoist.get_tasks().await {
            Ok(tasks) => {
                info!("✅ Fetched {} tasks from API", tasks.len());
                tasks
            }
            Err(e) => {
                error!("❌ Failed to fetch tasks: {e}");
                return Ok(SyncStatus::Error {
                    message: format!("Failed to fetch tasks: {e}"),
                });
            }
        };

        // Fetch all labels from API
        let labels = match self.todoist.get_labels().await {
            Ok(labels) => {
                info!("✅ Fetched {} labels from API", labels.len());
                labels
            }
            Err(e) => {
                error!("❌ Failed to fetch labels: {e}");
                return Ok(SyncStatus::Error {
                    message: format!("Failed to fetch labels: {e}"),
                });
            }
        };

        // Fetch all sections from API
        let sections = match self.todoist.get_sections().await {
            Ok(sections) => {
                info!("✅ Fetched {} sections from API", sections.len());
                sections
            }
            Err(e) => {
                error!("❌ Failed to fetch sections: {e}");
                info!("⚠️  Skipping sections sync due to API compatibility issue");
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
                info!("⚠️  No sections to store (skipped due to API issue)");
            }
        }

        Ok(SyncStatus::Success)
    }

    /// Look up remote_id from local task UUID.
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `task_uuid` - Local task UUID
    ///
    /// # Returns
    /// Remote task ID for Todoist API
    ///
    /// # Errors
    /// Returns error if task with given UUID doesn't exist locally
    async fn lookup_task_remote_id(
        conn: &sea_orm::DatabaseConnection,
        task_uuid: &Uuid,
    ) -> Result<String> {
        task::Entity::find()
            .filter(task::Column::Uuid.eq(*task_uuid))
            .one(conn)
            .await?
            .map(|t| t.remote_id)
            .ok_or_else(|| anyhow::anyhow!("Task not found: {}", task_uuid))
    }

    /// Look up remote_id from local project UUID.
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `project_uuid` - Local project UUID
    ///
    /// # Returns
    /// Remote project ID for Todoist API
    ///
    /// # Errors
    /// Returns error if project with given UUID doesn't exist locally
    async fn lookup_project_remote_id(
        conn: &sea_orm::DatabaseConnection,
        project_uuid: &Uuid,
    ) -> Result<String> {
        project::Entity::find()
            .filter(project::Column::Uuid.eq(*project_uuid))
            .one(conn)
            .await?
            .map(|p| p.remote_id)
            .ok_or_else(|| anyhow::anyhow!("Project not found: {}", project_uuid))
    }

    /// Look up remote_id from local label UUID.
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `label_uuid` - Local label UUID
    ///
    /// # Returns
    /// Remote label ID for Todoist API
    ///
    /// # Errors
    /// Returns error if label with given UUID doesn't exist locally
    async fn lookup_label_remote_id(
        conn: &sea_orm::DatabaseConnection,
        label_uuid: &Uuid,
    ) -> Result<String> {
        label::Entity::find()
            .filter(label::Column::Uuid.eq(*label_uuid))
            .one(conn)
            .await?
            .map(|l| l.remote_id)
            .ok_or_else(|| anyhow::anyhow!("Label not found: {}", label_uuid))
    }

    /// Look up local project UUID from remote project_id.
    ///
    /// # Arguments
    /// * `txn` - Database transaction
    /// * `remote_project_id` - Remote project ID from Todoist API
    /// * `context` - Context string for error message (e.g., "task creation", "section sync")
    ///
    /// # Returns
    /// Local project UUID
    ///
    /// # Errors
    /// Returns error if project with given remote_id doesn't exist locally
    async fn lookup_project_uuid(
        txn: &sea_orm::DatabaseTransaction,
        remote_project_id: &str,
        context: &str,
    ) -> Result<Uuid> {
        if let Some(project) = project::Entity::find()
            .filter(project::Column::RemoteId.eq(remote_project_id))
            .one(txn)
            .await?
        {
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
    /// * `remote_section_id` - Remote section ID from Todoist API
    ///
    /// # Returns
    /// Optional local section UUID (None if section_id is not provided)
    ///
    /// # Errors
    /// Returns error if database query fails
    async fn lookup_section_uuid(
        txn: &sea_orm::DatabaseTransaction,
        remote_section_id: Option<&String>,
    ) -> Result<Option<Uuid>> {
        if let Some(remote_id) = remote_section_id {
            let section_uuid = section::Entity::find()
                .filter(section::Column::RemoteId.eq(remote_id))
                .one(txn)
                .await?
                .map(|s| s.uuid);
            Ok(section_uuid)
        } else {
            Ok(None)
        }
    }

    /// Store projects in batch
    async fn store_projects_batch(&self, storage: &LocalStorage, projects: &[todoist_api::Project]) -> Result<()> {
        use sea_orm::sea_query::OnConflict;

        let txn = storage.conn.begin().await?;

        // First pass: Upsert all projects without parent_uuid relationships
        for api_project in projects {
            let local_project = project::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                remote_id: ActiveValue::Set(api_project.id.clone()),
                name: ActiveValue::Set(api_project.name.clone()),
                color: ActiveValue::Set(api_project.color.clone()),
                is_favorite: ActiveValue::Set(api_project.is_favorite),
                is_inbox_project: ActiveValue::Set(api_project.is_inbox_project),
                order_index: ActiveValue::Set(api_project.order),
                parent_uuid: ActiveValue::Set(None),
            };

            let mut insert = project::Entity::insert(local_project);
            insert = insert.on_conflict(
                OnConflict::column(project::Column::RemoteId)
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
        for api_project in projects {
            if let Some(remote_parent_id) = &api_project.parent_id {
                if let Some(parent) = project::Entity::find()
                    .filter(project::Column::RemoteId.eq(remote_parent_id))
                    .one(&txn)
                    .await?
                {
                    if let Some(project) = project::Entity::find()
                        .filter(project::Column::RemoteId.eq(&api_project.id))
                        .one(&txn)
                        .await?
                    {
                        let mut active_model: project::ActiveModel = project.into_active_model();
                        active_model.parent_uuid = ActiveValue::Set(Some(parent.uuid));
                        active_model.update(&txn).await?;
                    }
                }
            }
        }

        txn.commit().await?;
        Ok(())
    }

    /// Store labels in batch
    async fn store_labels_batch(&self, storage: &LocalStorage, labels: &[todoist_api::Label]) -> Result<()> {
        use sea_orm::sea_query::OnConflict;

        let txn = storage.conn.begin().await?;

        for api_label in labels {
            let local_label = label::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                remote_id: ActiveValue::Set(api_label.id.clone()),
                name: ActiveValue::Set(api_label.name.clone()),
                color: ActiveValue::Set(api_label.color.clone()),
                order_index: ActiveValue::Set(api_label.order),
                is_favorite: ActiveValue::Set(api_label.is_favorite),
            };

            let mut insert = label::Entity::insert(local_label);
            insert = insert.on_conflict(
                OnConflict::column(label::Column::RemoteId)
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
    async fn store_tasks_batch(&self, storage: &LocalStorage, tasks: &[todoist_api::Task]) -> Result<()> {
        use sea_orm::sea_query::OnConflict;

        let txn = storage.conn.begin().await?;

        // Track task labels for later processing
        let mut task_labels_map: Vec<(Uuid, Vec<String>)> = Vec::new();

        // First pass: Upsert all tasks without parent_uuid relationships
        for api_task in tasks {
            let label_names = api_task.labels.clone();

            // Look up local project UUID from remote project_id
            let project_uuid = match Self::lookup_project_uuid(&txn, &api_task.project_id, "task batch sync").await {
                Ok(uuid) => uuid,
                Err(_) => {
                    // Skip tasks whose projects don't exist locally (can happen with free tier API limitations)
                    continue;
                }
            };

            // Look up local section UUID from remote section_id if present
            let section_uuid = Self::lookup_section_uuid(&txn, api_task.section_id.as_ref()).await?;

            let duration_string = api_task.duration.as_ref().map(|d| match d.unit.as_str() {
                "minute" => format!("{}m", d.amount),
                "hour" => format!("{}h", d.amount),
                "day" => format!("{}d", d.amount),
                _ => format!("{} {}", d.amount, d.unit),
            });

            let local_task = task::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                remote_id: ActiveValue::Set(api_task.id.clone()),
                content: ActiveValue::Set(api_task.content.clone()),
                description: ActiveValue::Set(Some(api_task.description.clone())),
                project_uuid: ActiveValue::Set(project_uuid),
                section_uuid: ActiveValue::Set(section_uuid),
                parent_uuid: ActiveValue::Set(None),
                priority: ActiveValue::Set(api_task.priority),
                order_index: ActiveValue::Set(api_task.order),
                due_date: ActiveValue::Set(api_task.due.as_ref().map(|d| d.date.clone())),
                due_datetime: ActiveValue::Set(api_task.due.as_ref().and_then(|d| d.datetime.clone())),
                is_recurring: ActiveValue::Set(api_task.due.as_ref().is_some_and(|d| d.is_recurring)),
                deadline: ActiveValue::Set(api_task.deadline.as_ref().map(|d| d.date.clone())),
                duration: ActiveValue::Set(duration_string),
                is_completed: ActiveValue::Set(api_task.is_completed),
                is_deleted: ActiveValue::Set(false),
            };

            let mut insert = task::Entity::insert(local_task);
            insert = insert.on_conflict(
                OnConflict::column(task::Column::RemoteId)
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
            if let Some(task) = task::Entity::find()
                .filter(task::Column::RemoteId.eq(&api_task.id))
                .one(&txn)
                .await?
            {
                task_labels_map.push((task.uuid, label_names));
            }
        }

        // Second pass: Update parent_uuid references to use local UUIDs
        for api_task in tasks {
            if let Some(remote_parent_id) = &api_task.parent_id {
                if let Some(parent) = task::Entity::find()
                    .filter(task::Column::RemoteId.eq(remote_parent_id))
                    .one(&txn)
                    .await?
                {
                    if let Some(task) = task::Entity::find()
                        .filter(task::Column::RemoteId.eq(&api_task.id))
                        .one(&txn)
                        .await?
                    {
                        let mut active_model: task::ActiveModel = task.into_active_model();
                        active_model.parent_uuid = ActiveValue::Set(Some(parent.uuid));
                        active_model.update(&txn).await?;
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
                    if let Some(label) = label::Entity::find()
                        .filter(label::Column::Name.eq(&label_name))
                        .one(&txn)
                        .await?
                    {
                        let task_label_relation = task_label::ActiveModel {
                            task_uuid: ActiveValue::Set(task_uuid.clone()),
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
    async fn store_sections_batch(&self, storage: &LocalStorage, sections: &[todoist_api::Section]) -> Result<()> {
        use sea_orm::sea_query::OnConflict;

        let txn = storage.conn.begin().await?;

        for api_section in sections {
            // Look up local project UUID from remote project_id
            let project_uuid = Self::lookup_project_uuid(&txn, &api_section.project_id, "section sync").await?;

            let local_section = section::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                remote_id: ActiveValue::Set(api_section.id.clone()),
                name: ActiveValue::Set(api_section.name.clone()),
                project_uuid: ActiveValue::Set(project_uuid),
                order_index: ActiveValue::Set(api_section.order),
            };

            let mut insert = section::Entity::insert(local_section);
            insert = insert.on_conflict(
                OnConflict::column(section::Column::RemoteId)
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

    /// Marks a task as completed via the Todoist API and removes it from local storage.
    ///
    /// This method completes the task remotely (which automatically handles subtasks)
    /// and removes it from local storage since completed tasks are not displayed in the UI.
    /// Subtasks are automatically deleted via database CASCADE constraints.
    ///
    /// # Arguments
    /// * `task_uuid` - The local UUID of the task to complete
    ///
    /// # Errors
    /// Returns an error if the API call fails or local storage update fails
    pub async fn complete_task(&self, task_uuid: &Uuid) -> Result<()> {
        // Look up the task's remote_id for API call
        let storage = self.storage.lock().await;
        let remote_id = Self::lookup_task_remote_id(&storage.conn, task_uuid).await?;
        drop(storage); // Release lock before API call

        // Complete the task via API using remote_id (this handles subtasks automatically)
        self.todoist.complete_task(&remote_id).await?;

        // Then mark as completed in local storage (soft completion)
        let storage = self.storage.lock().await;

        let task = task::Entity::find()
            .filter(task::Column::Uuid.eq(*task_uuid))
            .one(&storage.conn)
            .await?;

        if let Some(task) = task {
            let mut active_model: task::ActiveModel = task.into_active_model();
            active_model.is_completed = ActiveValue::Set(true);
            active_model.update(&storage.conn).await?;
        }

        Ok(())
    }

    /// Permanently deletes a task via the Todoist API and removes it from local storage.
    ///
    /// This method performs a hard delete of the task both remotely and locally.
    /// The task will be permanently removed and cannot be recovered.
    ///
    /// # Arguments
    /// * `task_uuid` - The local UUID of the task to delete
    ///
    /// # Errors
    /// Returns an error if the API call fails or local storage update fails
    pub async fn delete_task(&self, task_uuid: &Uuid) -> Result<()> {
        // Look up the task's remote_id for API call
        let storage = self.storage.lock().await;
        let remote_id = Self::lookup_task_remote_id(&storage.conn, task_uuid).await?;
        drop(storage); // Release lock before API call

        // Delete the task via API using remote_id
        self.todoist.delete_task(&remote_id).await?;

        // Then mark as deleted in local storage (soft deletion)
        let storage = self.storage.lock().await;

        let task = task::Entity::find()
            .filter(task::Column::Uuid.eq(*task_uuid))
            .one(&storage.conn)
            .await?;

        if let Some(task) = task {
            let mut active_model: task::ActiveModel = task.into_active_model();
            active_model.is_deleted = ActiveValue::Set(true);
            active_model.update(&storage.conn).await?;
        }

        Ok(())
    }

    /// Restore a soft-deleted or completed task via the Todoist API and locally
    /// For completed tasks, reopens them. For deleted tasks, recreates them via API.
    pub async fn restore_task(&self, task_id: &Uuid) -> Result<()> {
        // First, get the task from local storage to check its state
        let storage = self.storage.lock().await;
        let task = task::Entity::find()
            .filter(task::Column::Uuid.eq(*task_id))
            .one(&storage.conn)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found in local storage: {}", task_id))?;

        if task.is_deleted {
            // For deleted tasks, we need to recreate them via API
            // Look up remote IDs before dropping storage lock
            let remote_project_id = Self::lookup_project_remote_id(&storage.conn, &task.project_uuid).await?;
            let remote_section_id = if let Some(section_uuid) = &task.section_uuid {
                // Look up the section's remote_id directly
                let section = section::Entity::find()
                    .filter(section::Column::Uuid.eq(*section_uuid))
                    .one(&storage.conn)
                    .await?;
                section.map(|s| s.remote_id)
            } else {
                None
            };
            let remote_parent_id = if let Some(parent_uuid) = &task.parent_uuid {
                Some(Self::lookup_task_remote_id(&storage.conn, parent_uuid).await?)
            } else {
                None
            };

            drop(storage); // Release the lock before API call

            // Create the task again via API
            let task_args = todoist_api::CreateTaskArgs {
                content: task.content.clone(),
                description: task.description.clone().filter(|d| !d.is_empty()),
                project_id: Some(remote_project_id),
                section_id: remote_section_id,
                parent_id: remote_parent_id,
                order: None,
                labels: None, // TODO: Fetch task labels from storage
                priority: Some(task.priority),
                due_string: task.due_date.clone(),
                due_date: None,
                due_datetime: task.due_datetime.clone(),
                due_lang: None,
                assignee_id: None,
                deadline_date: task.deadline.clone(),
                deadline_lang: None,
                duration: None,
                duration_unit: None,
            };

            let new_task = self.todoist.create_task(&task_args).await?;

            // Update local storage: remove the old soft-deleted task and add the new one
            let storage = self.storage.lock().await;

            // Hard delete the old soft-deleted task
            if let Some(old_task) = task::Entity::find()
                .filter(task::Column::Uuid.eq(*task_id))
                .one(&storage.conn)
                .await?
            {
                old_task.delete(&storage.conn).await?;
            }

            // Store the new task (reuse the single task upsert logic)
            let txn = storage.conn.begin().await?;

            let project_uuid = Self::lookup_project_uuid(&txn, &new_task.project_id, "task restore").await?;

            let section_uuid = Self::lookup_section_uuid(&txn, new_task.section_id.as_ref()).await?;

            let parent_uuid = if let Some(remote_parent_id) = &new_task.parent_id {
                task::Entity::find()
                    .filter(task::Column::RemoteId.eq(remote_parent_id))
                    .one(&txn)
                    .await?
                    .map(|t| t.uuid)
            } else {
                None
            };

            let duration_string = new_task.duration.as_ref().map(|d| match d.unit.as_str() {
                "minute" => format!("{}m", d.amount),
                "hour" => format!("{}h", d.amount),
                "day" => format!("{}d", d.amount),
                _ => format!("{} {}", d.amount, d.unit),
            });

            let local_task = task::ActiveModel {
                uuid: ActiveValue::Set(Uuid::new_v4()),
                remote_id: ActiveValue::Set(new_task.id),
                content: ActiveValue::Set(new_task.content),
                description: ActiveValue::Set(Some(new_task.description)),
                project_uuid: ActiveValue::Set(project_uuid),
                section_uuid: ActiveValue::Set(section_uuid),
                parent_uuid: ActiveValue::Set(parent_uuid),
                priority: ActiveValue::Set(new_task.priority),
                order_index: ActiveValue::Set(new_task.order),
                due_date: ActiveValue::Set(new_task.due.as_ref().map(|d| d.date.clone())),
                due_datetime: ActiveValue::Set(new_task.due.as_ref().and_then(|d| d.datetime.clone())),
                is_recurring: ActiveValue::Set(new_task.due.as_ref().is_some_and(|d| d.is_recurring)),
                deadline: ActiveValue::Set(new_task.deadline.as_ref().map(|d| d.date.clone())),
                duration: ActiveValue::Set(duration_string),
                is_completed: ActiveValue::Set(new_task.is_completed),
                is_deleted: ActiveValue::Set(false),
            };

            use sea_orm::sea_query::OnConflict;
            let mut insert = task::Entity::insert(local_task);
            insert = insert.on_conflict(
                OnConflict::column(task::Column::RemoteId)
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
            self.todoist.reopen_task(&remote_id).await?;

            // Clear local completion flag
            let storage = self.storage.lock().await;

            let task = task::Entity::find()
                .filter(task::Column::Uuid.eq(*task_id))
                .one(&storage.conn)
                .await?;

            if let Some(task) = task {
                let mut active_model: task::ActiveModel = task.into_active_model();
                active_model.is_completed = ActiveValue::Set(false);
                active_model.update(&storage.conn).await?;
            }
        }

        Ok(())
    }
}
