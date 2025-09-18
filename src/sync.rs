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
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::config::Config;
use crate::logger::Logger;
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
/// use terminalist::config::Config;
///
/// # async fn example() -> anyhow::Result<()> {
/// let config = Config::default();
/// let sync_service = SyncService::new("api_token".to_string(), false, &config).await?;
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
    logger: Option<Logger>,
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
    /// * `config` - Application configuration including logging settings
    ///
    /// # Returns
    /// A new `SyncService` instance ready for use
    ///
    /// # Errors
    /// Returns an error if local storage initialization fails
    pub async fn new(api_token: String, debug_mode: bool, config: &Config) -> Result<Self> {
        let todoist = TodoistWrapper::new(api_token);
        let storage = Arc::new(Mutex::new(LocalStorage::new(debug_mode).await?));
        let sync_in_progress = Arc::new(Mutex::new(false));

        // Initialize logger based on config
        let logger = match Logger::from_config(config.logging.enabled) {
            Ok(logger) => Some(logger),
            Err(e) => {
                eprintln!("Warning: Failed to initialize logging: {}", e);
                Some(Logger::new()) // Fallback to in-memory logging
            }
        };

        Ok(Self {
            todoist,
            storage,
            sync_in_progress,
            logger,
        })
    }

    /// Returns a clone of the logger if one is available.
    ///
    /// This method provides access to the service's logger for external logging needs.
    pub fn logger(&self) -> Option<Logger> {
        self.logger.clone()
    }

    /// Sets or replaces the logger for this sync service.
    ///
    /// # Arguments
    /// * `logger` - The new logger instance to use
    pub fn set_logger(&mut self, logger: Logger) {
        self.logger = Some(logger);
    }

    /// Log a message if logger is available
    fn log(&self, message: String) {
        if let Some(ref logger) = self.logger {
            logger.log(message);
        }
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
                id: local.id,
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
        self.log(format!("API: Creating label '{}' with color {:?}", name, color));

        // Create label via API using the CreateLabelArgs structure
        let label_args = todoist_api::CreateLabelArgs {
            name: name.to_string(),
            color: color.map(std::string::ToString::to_string),
            order: None,
            is_favorite: None,
        };
        let label = self.todoist.create_label(&label_args).await?;

        // Store the created label in local database immediately for UI refresh
        self.log(format!("Storage: Storing new label locally with ID {}", label.id));
        let storage = self.storage.lock().await;
        storage.store_single_label(label).await?;

        Ok(())
    }

    /// Update label content (name only for now)
    pub async fn update_label_content(&self, label_id: &str, name: &str) -> Result<()> {
        self.log(format!("API: Updating label name for ID {} to '{}'", label_id, name));

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
        self.log(format!(
            "Storage: Updating local label name for ID {} to '{}'",
            label_id, name
        ));
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
        self.log(format!(
            "API: Updating project name for ID {} to '{}'",
            project_id, name
        ));

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
        self.log(format!(
            "Storage: Updating local project name for ID {} to '{}'",
            project_id, name
        ));
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
        self.log(format!(
            "API: Updating task due date for ID {} to {:?}",
            task_id, due_date
        ));

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

        self.log(format!("API: Successfully updated task due date {}", task_id));
        Ok(())
    }

    /// Update task priority
    pub async fn update_task_priority(&self, task_id: &str, priority: i32) -> Result<()> {
        self.log(format!(
            "API: Updating task priority for ID {} to {}",
            task_id, priority
        ));

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

        self.log(format!("API: Successfully updated task priority {}", task_id));
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
        self.log("🔄 Starting sync process...".to_string());

        // Fetch projects from API
        let projects = match self.todoist.get_projects().await {
            Ok(projects) => {
                self.log(format!("✅ Fetched {} projects from API", projects.len()));
                projects
            }
            Err(e) => {
                self.log(format!("❌ Failed to fetch projects: {e}"));
                return Ok(SyncStatus::Error {
                    message: format!("Failed to fetch projects: {e}"),
                });
            }
        };

        // Fetch all tasks from API
        let tasks = match self.todoist.get_tasks().await {
            Ok(tasks) => {
                self.log(format!("✅ Fetched {} tasks from API", tasks.len()));
                tasks
            }
            Err(e) => {
                self.log(format!("❌ Failed to fetch tasks: {e}"));
                return Ok(SyncStatus::Error {
                    message: format!("Failed to fetch tasks: {e}"),
                });
            }
        };

        // Fetch all labels from API
        let labels = match self.todoist.get_labels().await {
            Ok(labels) => {
                self.log(format!("✅ Fetched {} labels from API", labels.len()));
                labels
            }
            Err(e) => {
                self.log(format!("❌ Failed to fetch labels: {e}"));
                return Ok(SyncStatus::Error {
                    message: format!("Failed to fetch labels: {e}"),
                });
            }
        };

        // Fetch all sections from API
        let sections = match self.todoist.get_sections().await {
            Ok(sections) => {
                self.log(format!("✅ Fetched {} sections from API", sections.len()));
                sections
            }
            Err(e) => {
                self.log(format!("❌ Failed to fetch sections: {e}"));
                self.log("⚠️  Skipping sections sync due to API compatibility issue".to_string());
                // For now, skip sections sync and continue with other data
                Vec::new()
            }
        };

        // Store in local database
        {
            let storage = self.storage.lock().await;
            self.log("💾 Storing data in local database...".to_string());

            if let Err(e) = storage.store_projects(projects).await {
                self.log(format!("❌ Failed to store projects: {e}"));
                return Ok(SyncStatus::Error {
                    message: format!("Failed to store projects: {e}"),
                });
            }
            self.log("✅ Stored projects in database".to_string());

            // Store labels BEFORE tasks so task-label relationships can be created
            if let Err(e) = storage.store_labels(labels).await {
                self.log(format!("❌ Failed to store labels: {e}"));
                return Ok(SyncStatus::Error {
                    message: format!("Failed to store labels: {e}"),
                });
            }
            self.log("✅ Stored labels in database".to_string());

            if let Err(e) = storage.store_tasks(tasks).await {
                self.log(format!("❌ Failed to store tasks: {e}"));
                return Ok(SyncStatus::Error {
                    message: format!("Failed to store tasks: {e}"),
                });
            }
            self.log("✅ Stored tasks in database".to_string());

            if !sections.is_empty() {
                if let Err(e) = storage.store_sections(sections).await {
                    self.log(format!("❌ Failed to store sections: {e}"));
                    return Ok(SyncStatus::Error {
                        message: format!("Failed to store sections: {e}"),
                    });
                }
                self.log("✅ Stored sections in database".to_string());
            } else {
                self.log("⚠️  No sections to store (skipped due to API issue)".to_string());
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

        // Then delete from local storage (subtasks deleted via CASCADE)
        // Since completed tasks aren't displayed, we can remove them entirely
        let storage = self.storage.lock().await;
        storage.delete_task(task_id).await?;
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

        // Then remove from local storage
        let storage = self.storage.lock().await;
        storage.delete_task(task_id).await?;

        Ok(())
    }
}
