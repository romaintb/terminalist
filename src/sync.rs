use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::debug_logger::DebugLogger;
use crate::storage::LocalStorage;
use crate::todoist::{CreateProjectArgs, LabelDisplay, ProjectDisplay, SectionDisplay, TaskDisplay, TodoistWrapper};

/// Sync service that manages data synchronization between API and local storage
#[derive(Clone)]
pub struct SyncService {
    todoist: TodoistWrapper,
    storage: Arc<Mutex<LocalStorage>>,
    sync_in_progress: Arc<Mutex<bool>>,
    debug_logger: Option<DebugLogger>,
}

#[derive(Debug, Clone)]
pub enum SyncStatus {
    Idle,
    InProgress,
    Success { last_sync: DateTime<Utc> },
    Error { message: String },
}

impl SyncService {
    /// Create a new sync service
    pub async fn new(api_token: String) -> Result<Self> {
        let todoist = TodoistWrapper::new(api_token);
        let storage = Arc::new(Mutex::new(LocalStorage::new().await?));
        let sync_in_progress = Arc::new(Mutex::new(false));

        Ok(Self {
            todoist,
            storage,
            sync_in_progress,
            debug_logger: None,
        })
    }

    /// Set the debug logger for this sync service
    pub fn set_debug_logger(&mut self, logger: DebugLogger) {
        self.debug_logger = Some(logger);
    }

    /// Log a debug message if logger is available
    fn log_debug(&self, message: String) {
        if let Some(ref logger) = self.debug_logger {
            logger.log(message);
        }
    }

    /// Get projects from local storage (fast)
    pub async fn get_projects(&self) -> Result<Vec<ProjectDisplay>> {
        let storage = self.storage.lock().await;
        storage.get_projects().await
    }

    /// Get tasks for a project from local storage (fast)
    pub async fn get_tasks_for_project(&self, project_id: &str) -> Result<Vec<TaskDisplay>> {
        let storage = self.storage.lock().await;
        storage.get_tasks_for_project(project_id).await
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

    /// Get tasks due today and overdue tasks from local storage (fast)
    pub async fn get_tasks_for_today(&self) -> Result<Vec<TaskDisplay>> {
        let storage = self.storage.lock().await;
        storage.get_tasks_for_today().await
    }

    /// Get tasks due tomorrow from local storage (fast)
    pub async fn get_tasks_for_tomorrow(&self) -> Result<Vec<TaskDisplay>> {
        let storage = self.storage.lock().await;
        storage.get_tasks_for_tomorrow().await
    }

    /// Check if sync is currently in progress
    pub async fn is_syncing(&self) -> bool {
        *self.sync_in_progress.lock().await
    }

    /// Check if we have local data available
    pub async fn has_local_data(&self) -> Result<bool> {
        let storage = self.storage.lock().await;
        storage.has_data().await
    }

    /// Get last sync time for projects
    pub async fn get_last_sync_time(&self) -> Result<Option<DateTime<Utc>>> {
        let storage = self.storage.lock().await;
        storage.get_last_sync("projects").await
    }

    /// Create a new project
    pub async fn create_project(&self, name: &str, parent_id: Option<&str>) -> Result<()> {
        // Create project via API using the new CreateProjectArgs structure
        let project_args = CreateProjectArgs {
            name: name.to_string(),
            color: None,
            parent_id: parent_id.map(std::string::ToString::to_string),
            is_favorite: None,
            view_style: None,
        };
        let _project = self.todoist.create_project(&project_args).await?;

        // The UI will handle the sync separately to ensure proper error handling
        Ok(())
    }

    /// Create a new task
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
        let _task = self.todoist.create_task(&task_args).await?;

        // The UI will handle the sync separately to ensure proper error handling
        Ok(())
    }

    /// Create a new label
    pub async fn create_label(&self, name: &str, color: Option<&str>) -> Result<()> {
        // Create label via API using the CreateLabelArgs structure
        let label_args = todoist_api::CreateLabelArgs {
            name: name.to_string(),
            color: color.map(std::string::ToString::to_string),
            order: None,
            is_favorite: None,
        };
        let _label = self.todoist.create_label(&label_args).await?;

        // The UI will handle the sync separately to ensure proper error handling
        Ok(())
    }

    /// Update label content (name only for now)
    pub async fn update_label_content(&self, label_id: &str, name: &str) -> Result<()> {
        // Update label via API using the UpdateLabelArgs structure
        let label_args = todoist_api::UpdateLabelArgs {
            name: Some(name.to_string()),
            // Set all other fields to None to avoid overwriting existing data
            color: None,
            order: None,
            is_favorite: None,
        };
        let _label = self.todoist.update_label(label_id, &label_args).await?;

        // The UI will handle the sync separately to ensure proper error handling
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
        // Update project via API using the UpdateProjectArgs structure
        let project_args = todoist_api::UpdateProjectArgs {
            name: Some(name.to_string()),
            // Set all other fields to None to avoid overwriting existing data
            color: None,
            is_favorite: None,
            view_style: None,
        };
        let _project = self
            .todoist
            .update_project(project_id, &project_args)
            .await?;

        // The UI will handle the sync separately to ensure proper error handling
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

    /// Delete a project
    pub async fn delete_project(&self, project_id: &str) -> Result<()> {
        // Delete project via API
        self.todoist.delete_project(project_id).await?;

        // Remove from local storage
        let storage = self.storage.lock().await;
        storage.delete_project(project_id).await?;

        Ok(())
    }

    /// Perform full sync with Todoist API
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
        self.log_debug("🔄 Starting sync process...".to_string());

        // Fetch projects from API
        let projects = match self.todoist.get_projects().await {
            Ok(projects) => {
                self.log_debug(format!("✅ Fetched {} projects from API", projects.len()));
                projects
            }
            Err(e) => {
                self.log_debug(format!("❌ Failed to fetch projects: {e}"));
                return Ok(SyncStatus::Error {
                    message: format!("Failed to fetch projects: {e}"),
                });
            }
        };

        // Fetch all tasks from API
        let tasks = match self.todoist.get_tasks().await {
            Ok(tasks) => {
                self.log_debug(format!("✅ Fetched {} tasks from API", tasks.len()));
                tasks
            }
            Err(e) => {
                self.log_debug(format!("❌ Failed to fetch tasks: {e}"));
                return Ok(SyncStatus::Error {
                    message: format!("Failed to fetch tasks: {e}"),
                });
            }
        };

        // Fetch all labels from API
        let labels = match self.todoist.get_labels().await {
            Ok(labels) => {
                self.log_debug(format!("✅ Fetched {} labels from API", labels.len()));
                labels
            }
            Err(e) => {
                self.log_debug(format!("❌ Failed to fetch labels: {e}"));
                return Ok(SyncStatus::Error {
                    message: format!("Failed to fetch labels: {e}"),
                });
            }
        };

        // Fetch all sections from API
        let sections = match self.todoist.get_sections().await {
            Ok(sections) => {
                self.log_debug(format!("✅ Fetched {} sections from API", sections.len()));
                sections
            }
            Err(e) => {
                self.log_debug(format!("❌ Failed to fetch sections: {e}"));
                self.log_debug("⚠️  Skipping sections sync due to API compatibility issue".to_string());
                // For now, skip sections sync and continue with other data
                Vec::new()
            }
        };

        // Store in local database
        {
            let storage = self.storage.lock().await;
            self.log_debug("💾 Storing data in local database...".to_string());

            if let Err(e) = storage.store_projects(projects).await {
                self.log_debug(format!("❌ Failed to store projects: {e}"));
                return Ok(SyncStatus::Error {
                    message: format!("Failed to store projects: {e}"),
                });
            }
            self.log_debug("✅ Stored projects in database".to_string());

            if let Err(e) = storage.store_tasks(tasks).await {
                self.log_debug(format!("❌ Failed to store tasks: {e}"));
                return Ok(SyncStatus::Error {
                    message: format!("Failed to store tasks: {e}"),
                });
            }
            self.log_debug("✅ Stored tasks in database".to_string());

            if let Err(e) = storage.store_labels(labels).await {
                self.log_debug(format!("❌ Failed to store labels: {e}"));
                return Ok(SyncStatus::Error {
                    message: format!("Failed to store labels: {e}"),
                });
            }
            self.log_debug("✅ Stored labels in database".to_string());

            if !sections.is_empty() {
                if let Err(e) = storage.store_sections(sections).await {
                    self.log_debug(format!("❌ Failed to store sections: {e}"));
                    return Ok(SyncStatus::Error {
                        message: format!("Failed to store sections: {e}"),
                    });
                }
                self.log_debug("✅ Stored sections in database".to_string());
            } else {
                self.log_debug("⚠️  No sections to store (skipped due to API issue)".to_string());
            }
        }

        Ok(SyncStatus::Success { last_sync: Utc::now() })
    }

    /// Check if sync is needed based on last sync time
    pub async fn should_sync(&self) -> Result<bool> {
        let last_sync = self.get_last_sync_time().await?;

        match last_sync {
            None => Ok(true), // Never synced
            Some(last) => {
                // Sync if last sync was more than 1 minute ago
                let threshold = Utc::now() - Duration::minutes(1);
                Ok(last < threshold)
            }
        }
    }

    /// Sync if needed (smart sync)
    pub async fn sync_if_needed(&self) -> Result<SyncStatus> {
        if self.should_sync().await? {
            self.sync().await
        } else {
            match self.get_last_sync_time().await? {
                Some(last_sync) => Ok(SyncStatus::Success { last_sync }),
                None => Ok(SyncStatus::Idle),
            }
        }
    }

    /// Force sync regardless of last sync time
    pub async fn force_sync(&self) -> Result<SyncStatus> {
        self.sync().await
    }

    /// Clear all local data (useful for reset)
    pub async fn clear_local_data(&self) -> Result<()> {
        let storage = self.storage.lock().await;
        storage.clear_all_data().await
    }

    /// Complete a task (mark as done)
    pub async fn complete_task(&self, task_id: &str) -> Result<()> {
        // First, complete the task via API
        self.todoist.complete_task(task_id).await?;

        // Then update local storage
        let storage = self.storage.lock().await;
        storage.mark_task_completed(task_id).await?;

        Ok(())
    }

    /// Reopen a task (mark as incomplete)
    pub async fn reopen_task(&self, task_id: &str) -> Result<()> {
        // First, reopen the task via API
        self.todoist.reopen_task(task_id).await?;

        // Then update local storage
        let storage = self.storage.lock().await;
        storage.mark_task_incomplete(task_id).await?;

        Ok(())
    }

    /// Delete a task permanently
    pub async fn delete_task(&self, task_id: &str) -> Result<()> {
        // First, delete the task via API
        self.todoist.delete_task(task_id).await?;

        // Then remove from local storage
        let storage = self.storage.lock().await;
        storage.delete_task(task_id).await?;

        Ok(())
    }

    /// Get sync statistics
    pub async fn get_sync_stats(&self) -> Result<SyncStats> {
        let storage = self.storage.lock().await;
        let project_count = storage.get_projects().await?.len();
        let task_count = storage.get_all_tasks().await?.len();
        let last_sync = storage.get_last_sync("projects").await?;

        Ok(SyncStats {
            project_count,
            task_count,
            last_sync,
            has_data: project_count > 0,
        })
    }
}

/// Statistics about local data and sync status
#[derive(Debug, Clone)]
pub struct SyncStats {
    pub project_count: usize,
    pub task_count: usize,
    pub last_sync: Option<DateTime<Utc>>,
    pub has_data: bool,
}

impl SyncStats {
    /// Get a human-readable description of sync status
    #[must_use]
    pub fn status_description(&self) -> String {
        if self.has_data {
            match &self.last_sync {
                Some(last) => {
                    let elapsed = Utc::now() - *last;
                    if elapsed.num_minutes() < 1 {
                        "Just synced".to_string()
                    } else if elapsed.num_minutes() < 60 {
                        format!("Synced {elapsed} minutes ago", elapsed = elapsed.num_minutes())
                    } else if elapsed.num_hours() < 24 {
                        format!("Synced {elapsed} hours ago", elapsed = elapsed.num_hours())
                    } else {
                        format!("Synced {elapsed} days ago", elapsed = elapsed.num_days())
                    }
                }
                None => "Never synced".to_string(),
            }
        } else {
            "No local data - sync needed".to_string()
        }
    }

    /// Get a summary of local data
    #[must_use]
    pub fn data_summary(&self) -> String {
        format!("{} projects, {} tasks", self.project_count, self.task_count)
    }
}
