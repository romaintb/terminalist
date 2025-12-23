use crate::entities::task;
use crate::repositories::{ProjectRepository, SectionRepository, TaskRepository};
use crate::sync::SyncService;
use crate::utils::datetime;
use anyhow::Result;
use sea_orm::{ActiveValue, EntityTrait, IntoActiveModel, TransactionTrait};
use uuid::Uuid;

impl SyncService {
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
        let remote_project_id = {
            let storage = self.storage.lock().await;
            if let Some(uuid) = project_uuid {
                Some(ProjectRepository::get_remote_id(&storage.conn, &uuid).await?)
            } else {
                None
            }
            // Lock is automatically dropped here when storage goes out of scope
        };

        // Create task via backend using backend CreateTaskArgs (lock is not held)
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
        let backend_task = self
            .get_backend()
            .await?
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
        let section_uuid =
            Self::lookup_section_uuid(&txn, &self.backend_uuid, backend_task.section_remote_id.as_ref()).await?;

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
        let _task = self
            .get_backend()
            .await?
            .update_task(&remote_id, task_args)
            .await
            .map_err(|e| anyhow::anyhow!("Backend error: {}", e))?;

        // Update local storage immediately after successful backend call
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
        let _task = self
            .get_backend()
            .await?
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

        Ok(())
    }

    /// Update task priority
    pub async fn update_task_priority(&self, task_uuid: &Uuid, priority: i32) -> Result<()> {
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
        let _task = self
            .get_backend()
            .await?
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

        Ok(())
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

            let new_task = self
                .get_backend()
                .await?
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

            let project_uuid =
                Self::lookup_project_uuid(&txn, &self.backend_uuid, &new_task.project_remote_id, "task restore")
                    .await?;

            let section_uuid =
                Self::lookup_section_uuid(&txn, &self.backend_uuid, new_task.section_remote_id.as_ref()).await?;

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
