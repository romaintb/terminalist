use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::storage::LocalStorage;
use crate::todoist::{CreateProjectArgs, LabelDisplay, ProjectDisplay, TaskDisplay, TodoistWrapper};

/// Sync service that manages data synchronization between API and local storage
#[derive(Clone)]
pub struct SyncService {
    todoist: TodoistWrapper,
    storage: Arc<Mutex<LocalStorage>>,
    sync_in_progress: Arc<Mutex<bool>>,
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
        })
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
        // Fetch projects from API
        let projects = match self.todoist.get_projects().await {
            Ok(projects) => projects,
            Err(e) => {
                return Ok(SyncStatus::Error {
                    message: format!("Failed to fetch projects: {e}"),
                });
            }
        };

        // Fetch all tasks from API
        let tasks = match self.todoist.get_tasks().await {
            Ok(tasks) => tasks,
            Err(e) => {
                return Ok(SyncStatus::Error {
                    message: format!("Failed to fetch tasks: {e}"),
                });
            }
        };

        // Fetch all labels from API
        let labels = match self.todoist.get_labels().await {
            Ok(labels) => labels,
            Err(e) => {
                return Ok(SyncStatus::Error {
                    message: format!("Failed to fetch labels: {e}"),
                });
            }
        };

        // Store in local database
        {
            let storage = self.storage.lock().await;

            if let Err(e) = storage.store_projects(projects).await {
                return Ok(SyncStatus::Error {
                    message: format!("Failed to store projects: {e}"),
                });
            }

            if let Err(e) = storage.store_tasks(tasks).await {
                return Ok(SyncStatus::Error {
                    message: format!("Failed to store tasks: {e}"),
                });
            }

            if let Err(e) = storage.store_labels(labels).await {
                return Ok(SyncStatus::Error {
                    message: format!("Failed to store labels: {e}"),
                });
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
