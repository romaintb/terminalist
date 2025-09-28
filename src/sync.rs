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
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::storage::LocalStorage;
use crate::todoist::{CreateProjectArgs, LabelDisplay, ProjectDisplay, SectionDisplay, TaskDisplay, TodoistWrapper};
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

        // Register the default "todoist" backend for compatibility with new schema
        {
            let storage_lock = storage.lock().await;
            storage_lock.register_backend(
                "todoist",
                "todoist",
                "Todoist",
                true, // enabled
                None  // no config stored in DB
            ).await?;
        }

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
    /// A vector of `ProjectDisplay` objects representing all available projects
    ///
    /// # Note
    /// As of 2025, Todoist allows free plan users to create more than 5 projects via the API,
    /// but the GET /projects API endpoint will only return the first 5 projects for free users.
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_projects(&self) -> Result<Vec<ProjectDisplay>> {
        let storage = self.storage.lock().await;
        storage.get_projects().await
    }

    /// Retrieves all tasks for a specific project from local storage.
    ///
    /// # Arguments
    /// * `project_id` - The unique identifier of the project
    ///
    /// # Returns
    /// A vector of `TaskDisplay` objects for the specified project
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_tasks_for_project(&self, project_id: &str) -> Result<Vec<TaskDisplay>> {
        let storage = self.storage.lock().await;
        storage.get_tasks_for_project(project_id).await
    }

    /// Retrieves all tasks from local storage across all projects.
    ///
    /// This method is primarily used for search functionality and global task operations.
    /// It provides fast access to the complete task dataset.
    ///
    /// # Returns
    /// A vector of all `TaskDisplay` objects in the local database
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_all_tasks(&self) -> Result<Vec<TaskDisplay>> {
        let storage = self.storage.lock().await;
        storage.get_all_tasks().await
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
    /// A vector of `TaskDisplay` objects matching the search criteria
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn search_tasks(&self, query: &str) -> Result<Vec<TaskDisplay>> {
        let storage = self.storage.lock().await;
        storage.search_tasks(query).await
    }

    /// Get all labels from local storage (fast)
    pub async fn get_labels(&self) -> Result<Vec<LabelDisplay>> {
        let storage = self.storage.lock().await;
        let local_labels = storage.get_all_labels().await?;

        // Convert LocalLabel to LabelDisplay
        let labels = local_labels
            .into_iter()
            .map(|local| LabelDisplay {
                uuid: local.uuid,
                name: local.name,
                color: local.color,
            })
            .collect();

        Ok(labels)
    }

    /// Get all sections from local storage (fast)
    pub async fn get_sections(&self) -> Result<Vec<SectionDisplay>> {
        let storage = self.storage.lock().await;
        storage.get_sections().await
    }

    /// Get sections for a project from local storage (fast)
    pub async fn get_sections_for_project(&self, project_id: &str) -> Result<Vec<SectionDisplay>> {
        let storage = self.storage.lock().await;
        storage.get_sections_for_project(project_id).await
    }

    /// Get tasks with a specific label from local storage (fast)
    pub async fn get_tasks_with_label(&self, label_name: &str) -> Result<Vec<TaskDisplay>> {
        let storage = self.storage.lock().await;
        let all_tasks = storage.get_all_tasks().await?;

        // Filter tasks that have the specified label
        let filtered_tasks = all_tasks
            .into_iter()
            .filter(|task| task.labels.iter().any(|label| label.name == label_name))
            .collect();

        Ok(filtered_tasks)
    }

    /// Retrieves tasks for the "Today" view with business logic.
    ///
    /// This method implements the UI business logic for the Today view by combining
    /// overdue tasks with tasks due today. Overdue tasks are shown first, followed
    /// by today's tasks.
    ///
    /// # Returns
    /// A vector of `TaskDisplay` objects for the Today view, with overdue tasks first
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_tasks_for_today(&self) -> Result<Vec<TaskDisplay>> {
        let storage = self.storage.lock().await;
        let today = datetime::format_today();

        let overdue_tasks = storage.get_overdue_tasks().await?;
        let today_tasks = storage.get_tasks_due_on(&today).await?;

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
    /// A vector of `TaskDisplay` objects due tomorrow
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_tasks_for_tomorrow(&self) -> Result<Vec<TaskDisplay>> {
        let storage = self.storage.lock().await;
        let tomorrow = datetime::format_date_with_offset(1);

        storage.get_tasks_due_on(&tomorrow).await
    }

    /// Retrieves tasks for the "Upcoming" view with business logic.
    ///
    /// This method implements the UI business logic for the Upcoming view by combining
    /// overdue tasks, today's tasks, and tasks due within the next 3 months.
    /// Tasks are ordered as: overdue → today → future (next 3 months).
    ///
    /// # Returns
    /// A vector of `TaskDisplay` objects for the Upcoming view, properly ordered
    ///
    /// # Errors
    /// Returns an error if local storage access fails
    pub async fn get_tasks_for_upcoming(&self) -> Result<Vec<TaskDisplay>> {
        let storage = self.storage.lock().await;
        let today = datetime::format_today();
        let three_months_later = datetime::format_date_with_offset(90);

        let overdue_tasks = storage.get_overdue_tasks().await?;
        let today_tasks = storage.get_tasks_due_on(&today).await?;
        let future_tasks = storage.get_tasks_due_between(&today, &three_months_later).await?;

        // UI business rule: overdue → today → future
        let mut result = overdue_tasks;
        result.extend(today_tasks);
        result.extend(future_tasks.into_iter().filter(|task| {
            // Remove today tasks from future to avoid duplicates
            if let Some(due_date) = &task.due {
                due_date != &today
            } else {
                true
            }
        }));
        Ok(result)
    }

    /// Get a single task by ID from local storage (fast)
    pub async fn get_task_by_id(&self, task_id: &str) -> Result<Option<TaskDisplay>> {
        let storage = self.storage.lock().await;
        storage.get_task_by_id(task_id).await
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
    /// * `parent_id` - Optional parent project ID for creating sub-projects
    ///
    /// # Note
    /// As of 2025, Todoist allows free plan users to create more than 5 projects via the API,
    /// but the GET /projects API endpoint will only return the first 5 projects for free users.
    ///
    /// # Errors
    /// Returns an error if the API call fails or local storage update fails
    pub async fn create_project(&self, name: &str, parent_id: Option<&str>) -> Result<()> {
        // Create project via API using the new CreateProjectArgs structure
        let project_args = CreateProjectArgs {
            name: name.to_string(),
            color: None,
            parent_id: parent_id.map(std::string::ToString::to_string),
            is_favorite: None,
            view_style: None,
        };
        let project = self.todoist.create_project(&project_args).await?;

        // Store the created project in local database immediately for UI refresh
        let storage = self.storage.lock().await;
        storage.store_single_project(project).await?;

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
    /// * `project_id` - Optional project ID to assign the task to a specific project
    ///
    /// # Errors
    /// Returns an error if the API call fails or local storage update fails
    pub async fn create_task(&self, content: &str, project_id: Option<&str>) -> Result<()> {
        // Create task via API using the new CreateTaskArgs structure
        let task_args = todoist_api::CreateTaskArgs {
            content: content.to_string(),
            description: None,
            project_id: project_id.map(std::string::ToString::to_string),
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
        let task = self.todoist.create_task(&task_args).await?;

        // Store the created task in local database immediately for UI refresh
        let storage = self.storage.lock().await;
        storage.store_single_task(task).await?;

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
        let label = self.todoist.create_label(&label_args).await?;

        // Store the created label in local database immediately for UI refresh
        info!("Storage: Storing new label locally with ID {}", label.id);
        let storage = self.storage.lock().await;
        storage.store_single_label(label).await?;

        Ok(())
    }

    /// Update label content (name only for now)
    pub async fn update_label_content(&self, label_id: &str, name: &str) -> Result<()> {
        info!("API: Updating label name for ID {} to '{}'", label_id, name);

        // Update label via API using the UpdateLabelArgs structure
        let label_args = todoist_api::UpdateLabelArgs {
            name: Some(name.to_string()),
            // Set all other fields to None to avoid overwriting existing data
            color: None,
            order: None,
            is_favorite: None,
        };
        let _label = self.todoist.update_label(label_id, &label_args).await?;

        // Update local storage immediately after successful API call
        info!("Storage: Updating local label name for ID {} to '{}'", label_id, name);
        let storage = self.storage.lock().await;
        storage.update_label_name(label_id, name).await?;

        Ok(())
    }

    /// Delete a label
    pub async fn delete_label(&self, label_id: &str) -> Result<()> {
        // Delete label via API
        self.todoist.delete_label(label_id).await?;

        // Note: Local storage deletion will be handled by the next sync
        Ok(())
    }

    /// Update project content (name only for now)
    pub async fn update_project_content(&self, project_id: &str, name: &str) -> Result<()> {
        info!("API: Updating project name for ID {} to '{}'", project_id, name);

        // Update project via API using the UpdateProjectArgs structure
        let project_args = todoist_api::UpdateProjectArgs {
            name: Some(name.to_string()),
            // Set all other fields to None to avoid overwriting existing data
            color: None,
            is_favorite: None,
            view_style: None,
        };
        let _project = self.todoist.update_project(project_id, &project_args).await?;

        // Update local storage immediately after successful API call
        info!(
            "Storage: Updating local project name for ID {} to '{}'",
            project_id, name
        );
        let storage = self.storage.lock().await;
        storage.update_project_name(project_id, name).await?;

        Ok(())
    }

    /// Update task content
    pub async fn update_task_content(&self, task_id: &str, content: &str) -> Result<()> {
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
        let _task = self.todoist.update_task(task_id, &task_args).await?;

        // The UI will handle the sync separately to ensure proper error handling
        Ok(())
    }

    /// Update task due date
    pub async fn update_task_due_date(&self, task_id: &str, due_date: Option<&str>) -> Result<()> {
        info!("API: Updating task due date for ID {} to {:?}", task_id, due_date);

        // First, update task via API using the UpdateTaskArgs structure
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
        let _task = self.todoist.update_task(task_id, &task_args).await?;

        // Then update local storage
        let storage = self.storage.lock().await;
        storage.update_task_due_date(task_id, due_date).await?;

        info!("API: Successfully updated task due date {}", task_id);
        Ok(())
    }

    /// Update task priority
    pub async fn update_task_priority(&self, task_id: &str, priority: i32) -> Result<()> {
        info!("API: Updating task priority for ID {} to {}", task_id, priority);

        // First, update task via API using the UpdateTaskArgs structure
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
        let _task = self.todoist.update_task(task_id, &task_args).await?;

        // Then update local storage
        let storage = self.storage.lock().await;
        storage.update_task_priority(task_id, priority).await?;

        info!("API: Successfully updated task priority {}", task_id);
        Ok(())
    }

    /// Delete a project
    pub async fn delete_project(&self, project_id: &str) -> Result<()> {
        // Delete project via API
        self.todoist.delete_project(project_id).await?;

        // Remove from local storage
        let storage = self.storage.lock().await;
        storage.delete_project(project_id).await?;

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

            if let Err(e) = storage.store_projects(projects).await {
                error!("❌ Failed to store projects: {e}");
                return Ok(SyncStatus::Error {
                    message: format!("Failed to store projects: {e}"),
                });
            }
            info!("✅ Stored projects in database");

            // Store labels BEFORE tasks so task-label relationships can be created
            if let Err(e) = storage.store_labels(labels).await {
                error!("❌ Failed to store labels: {e}");
                return Ok(SyncStatus::Error {
                    message: format!("Failed to store labels: {e}"),
                });
            }
            info!("✅ Stored labels in database");

            if let Err(e) = storage.store_tasks(tasks).await {
                error!("❌ Failed to store tasks: {e}");
                return Ok(SyncStatus::Error {
                    message: format!("Failed to store tasks: {e}"),
                });
            }
            info!("✅ Stored tasks in database");

            if !sections.is_empty() {
                if let Err(e) = storage.store_sections(sections).await {
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
    /// * `task_id` - The unique identifier of the task to complete
    ///
    /// # Errors
    /// Returns an error if the API call fails or local storage update fails
    pub async fn complete_task(&self, task_id: &str) -> Result<()> {
        // First, complete the task via API (this handles subtasks automatically)
        self.todoist.complete_task(task_id).await?;

        // Then mark as completed in local storage (soft completion)
        // This allows us to show completed tasks and recover from accidental completions
        let storage = self.storage.lock().await;
        storage.mark_task_completed(task_id).await?;
        drop(storage);

        Ok(())
    }

    /// Permanently deletes a task via the Todoist API and removes it from local storage.
    ///
    /// This method performs a hard delete of the task both remotely and locally.
    /// The task will be permanently removed and cannot be recovered.
    ///
    /// # Arguments
    /// * `task_id` - The unique identifier of the task to delete
    ///
    /// # Errors
    /// Returns an error if the API call fails or local storage update fails
    pub async fn delete_task(&self, task_id: &str) -> Result<()> {
        // First, delete the task via API
        self.todoist.delete_task(task_id).await?;

        // Then mark as deleted in local storage (soft deletion)
        // This allows recovery from accidental deletions
        let storage = self.storage.lock().await;
        storage.mark_task_deleted(task_id).await?;

        Ok(())
    }

    /// Restore a soft-deleted or completed task via the Todoist API and locally
    /// For completed tasks, reopens them. For deleted tasks, recreates them via API.
    pub async fn restore_task(&self, task_id: &str) -> Result<()> {
        // First, get the task from local storage to check its state
        let storage = self.storage.lock().await;
        let task = storage
            .get_task_by_id(task_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found in local storage: {}", task_id))?;

        if task.is_deleted {
            // For deleted tasks, we need to recreate them via API since they were permanently deleted
            drop(storage); // Release the lock before API call

            // Create the task again via API
            let task_args = todoist_api::CreateTaskArgs {
                content: task.content.clone(),
                description: if task.description.is_empty() {
                    None
                } else {
                    Some(task.description.clone())
                },
                project_id: Some(task.project_id.clone()),
                section_id: task.section_id.clone(),
                parent_id: task.parent_uuid.clone(),
                order: None,
                labels: Some(task.labels.iter().map(|l| l.name.clone()).collect()),
                priority: Some(task.priority),
                due_string: task.due.clone(),
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
            storage.delete_task(task_id).await?; // Hard delete the old soft-deleted task
            storage.store_single_task(new_task).await?; // Store the new task
        } else {
            // For completed tasks, just reopen them
            drop(storage); // Release the lock before API call
            self.todoist.reopen_task(task_id).await?;

            // Clear local completion flag
            let storage = self.storage.lock().await;
            storage.restore_task(task_id).await?;
        }

        Ok(())
    }
}
