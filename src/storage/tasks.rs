use anyhow::Result;
use sea_orm::{prelude::*, ActiveValue, IntoActiveModel, QueryOrder, QuerySelect};

use crate::entities::{label, task, task_label};
use crate::todoist::{LabelDisplay, Task as TodoistTask, TaskDisplay};

use super::LocalStorage;

impl LocalStorage {
    /// Store a task in the database
    pub async fn store_task(&self, task_data: &TodoistTask) -> Result<()> {
        // First, find the project UUID from remote_id
        let project = self
            .get_project_by_remote_id(&task_data.project_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Project not found: {}", task_data.project_id))?;

        // Find section UUID if section_id is present
        let section_uuid = if let Some(section_id) = &task_data.section_id {
            self.get_section_by_remote_id(section_id)
                .await?
                .map(|s| s.uuid)
        } else {
            None
        };

        // Find parent task UUID if parent_id is present
        let parent_uuid = if let Some(parent_id) = &task_data.parent_id {
            self.get_task_by_remote_id(parent_id)
                .await?
                .map(|t| t.uuid)
        } else {
            None
        };

        let model = task::ActiveModel {
            uuid: ActiveValue::Set(Uuid::new_v4().to_string()),
            remote_id: ActiveValue::Set(task_data.id.clone()),
            content: ActiveValue::Set(task_data.content.clone()),
            description: ActiveValue::Set(Some(task_data.description.clone())),
            priority: ActiveValue::Set(task_data.priority),
            order_index: ActiveValue::Set(task_data.order),
            is_completed: ActiveValue::Set(task_data.is_completed),
            project_uuid: ActiveValue::Set(project.uuid),
            section_uuid: ActiveValue::Set(section_uuid),
            parent_uuid: ActiveValue::Set(parent_uuid),
            due_date: ActiveValue::Set(task_data.due.as_ref().map(|d| d.date.clone())),
            due_datetime: ActiveValue::Set(task_data.due.as_ref().and_then(|d| d.datetime.clone())),
            is_recurring: ActiveValue::Set(task_data.due.as_ref().map(|d| d.is_recurring).unwrap_or(false)),
            deadline: ActiveValue::Set(task_data.deadline.as_ref().map(|d| d.date.clone())),
            duration: ActiveValue::Set(task_data.duration.as_ref().map(|d| serde_json::to_string(d).ok()).flatten()),
            is_deleted: ActiveValue::Set(false),
        };

        let insert_result = task::Entity::insert(model)
            .on_conflict(
                sea_orm::sea_query::OnConflict::column(task::Column::RemoteId)
                    .update_columns([
                        task::Column::Content,
                        task::Column::Description,
                        task::Column::Priority,
                        task::Column::OrderIndex,
                        task::Column::IsCompleted,
                        task::Column::ProjectUuid,
                        task::Column::SectionUuid,
                        task::Column::ParentUuid,
                        task::Column::DueDate,
                    ])
                    .to_owned(),
            )
            .exec(&self.conn)
            .await?;

        // Get the task UUID (either from insert or existing)
        let task_uuid = if let Some(existing_task) = self.get_task_by_remote_id(&task_data.id).await? {
            existing_task.uuid
        } else {
            insert_result.last_insert_id
        };

        // Handle task labels
        if !task_data.labels.is_empty() {
            // Delete existing task-label associations
            task_label::Entity::delete_many()
                .filter(task_label::Column::TaskUuid.eq(&task_uuid))
                .exec(&self.conn)
                .await?;

            // Insert new associations
            for label_id in &task_data.labels {
                if let Some(label) = self.get_label_by_remote_id(label_id).await? {
                    let task_label_model = task_label::ActiveModel {
                        task_uuid: ActiveValue::Set(task_uuid.clone()),
                        label_uuid: ActiveValue::Set(label.uuid),
                    };

                    task_label::Entity::insert(task_label_model)
                        .exec(&self.conn)
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// Get all tasks for a project
    pub async fn get_tasks_by_project_uuid(&self, project_uuid: &str) -> Result<Vec<task::Model>> {
        let tasks = task::Entity::find()
            .filter(task::Column::ProjectUuid.eq(project_uuid))
            .filter(task::Column::IsCompleted.eq(false))
            .order_by_asc(task::Column::OrderIndex)
            .all(&self.conn)
            .await?;

        Ok(tasks)
    }

    /// Get all tasks for a section
    pub async fn get_tasks_by_section_uuid(&self, section_uuid: &str) -> Result<Vec<task::Model>> {
        let tasks = task::Entity::find()
            .filter(task::Column::SectionUuid.eq(section_uuid))
            .filter(task::Column::IsCompleted.eq(false))
            .order_by_asc(task::Column::OrderIndex)
            .all(&self.conn)
            .await?;

        Ok(tasks)
    }

    /// Get all root tasks for a project (tasks without section and without parent)
    pub async fn get_root_tasks_by_project_uuid(&self, project_uuid: &str) -> Result<Vec<task::Model>> {
        let tasks = task::Entity::find()
            .filter(task::Column::ProjectUuid.eq(project_uuid))
            .filter(task::Column::SectionUuid.is_null())
            .filter(task::Column::ParentUuid.is_null())
            .filter(task::Column::IsCompleted.eq(false))
            .order_by_asc(task::Column::OrderIndex)
            .all(&self.conn)
            .await?;

        Ok(tasks)
    }

    /// Get all subtasks of a parent task
    pub async fn get_subtasks(&self, parent_uuid: &str) -> Result<Vec<task::Model>> {
        let tasks = task::Entity::find()
            .filter(task::Column::ParentUuid.eq(parent_uuid))
            .filter(task::Column::IsCompleted.eq(false))
            .order_by_asc(task::Column::OrderIndex)
            .all(&self.conn)
            .await?;

        Ok(tasks)
    }

    /// Get a task by its remote ID
    pub async fn get_task_by_remote_id(&self, remote_id: &str) -> Result<Option<task::Model>> {
        let task = task::Entity::find()
            .filter(task::Column::RemoteId.eq(remote_id))
            .one(&self.conn)
            .await?;

        Ok(task)
    }

    /// Get a task by its UUID
    pub async fn get_task_by_uuid(&self, uuid: &str) -> Result<Option<task::Model>> {
        let task = task::Entity::find()
            .filter(task::Column::Uuid.eq(uuid))
            .one(&self.conn)
            .await?;

        Ok(task)
    }

    /// Get all label UUIDs for a task
    pub async fn get_task_label_uuids(&self, task_uuid: &str) -> Result<Vec<String>> {
        let task_labels = task_label::Entity::find()
            .filter(task_label::Column::TaskUuid.eq(task_uuid))
            .all(&self.conn)
            .await?;

        Ok(task_labels.into_iter().map(|tl| tl.label_uuid).collect())
    }

    /// Get all labels for a task as LabelDisplay
    pub async fn get_task_labels(&self, task_uuid: &str) -> Result<Vec<LabelDisplay>> {
        let label_uuids = self.get_task_label_uuids(task_uuid).await?;

        let mut labels = Vec::new();
        for label_uuid in label_uuids {
            if let Some(label_model) = label::Entity::find()
                .filter(label::Column::Uuid.eq(&label_uuid))
                .one(&self.conn)
                .await?
            {
                labels.push(LabelDisplay {
                    id: label_model.uuid,
                    name: label_model.name,
                    color: label_model.color,
                });
            }
        }

        Ok(labels)
    }

    /// Convert a task model to TaskDisplay
    async fn task_to_display(&self, task: task::Model) -> Result<TaskDisplay> {
        let labels = self.get_task_labels(&task.uuid).await?;

        Ok(TaskDisplay {
            id: task.uuid,
            content: task.content,
            project_id: task.project_uuid,
            section_id: task.section_uuid,
            parent_id: task.parent_uuid,
            priority: task.priority,
            due: task.due_date,
            due_datetime: task.due_datetime,
            is_recurring: task.is_recurring,
            deadline: task.deadline,
            duration: task.duration,
            labels,
            description: task.description.unwrap_or_default(),
            is_completed: task.is_completed,
            is_deleted: task.is_deleted,
        })
    }

    /// Delete all tasks
    pub async fn delete_all_tasks(&self) -> Result<()> {
        task::Entity::delete_many().exec(&self.conn).await?;
        Ok(())
    }

    /// Update task parent relationships after all tasks are stored
    pub async fn update_task_parents(&self, tasks: &[TodoistTask]) -> Result<()> {
        for task_data in tasks {
            if let Some(parent_id) = &task_data.parent_id {
                // Find the parent task's UUID
                if let Some(parent) = self.get_task_by_remote_id(parent_id).await? {
                    // Find the current task and update its parent_uuid
                    if let Some(current) = self.get_task_by_remote_id(&task_data.id).await? {
                        let mut active_model: task::ActiveModel = current.into_active_model();
                        active_model.parent_uuid = ActiveValue::Set(Some(parent.uuid));
                        active_model.update(&self.conn).await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// Get all tasks (including completed)
    async fn get_all_task_models(&self) -> Result<Vec<task::Model>> {
        let tasks = task::Entity::find()
            .filter(task::Column::IsDeleted.eq(false))
            .order_by_asc(task::Column::OrderIndex)
            .all(&self.conn)
            .await?;

        Ok(tasks)
    }

    /// Get all tasks as TaskDisplay (for UI)
    pub async fn get_all_tasks(&self) -> Result<Vec<TaskDisplay>> {
        let tasks = self.get_all_task_models().await?;
        let mut display_tasks = Vec::new();
        for task in tasks {
            display_tasks.push(self.task_to_display(task).await?);
        }
        Ok(display_tasks)
    }

    /// Get tasks for a project as TaskDisplay (for UI)
    pub async fn get_tasks_for_project(&self, project_uuid: &str) -> Result<Vec<TaskDisplay>> {
        let tasks = self.get_tasks_by_project_uuid(project_uuid).await?;
        let mut display_tasks = Vec::new();
        for task in tasks {
            display_tasks.push(self.task_to_display(task).await?);
        }
        Ok(display_tasks)
    }

    /// Search tasks by content
    pub async fn search_tasks(&self, query: &str) -> Result<Vec<TaskDisplay>> {
        use sea_orm::sea_query::Expr;

        let tasks = task::Entity::find()
            .filter(task::Column::IsDeleted.eq(false))
            .filter(task::Column::IsCompleted.eq(false))
            .filter(
                Expr::col(task::Column::Content)
                    .like(format!("%{}%", query))
                    .or(Expr::col(task::Column::Description)
                        .like(format!("%{}%", query)))
            )
            .order_by_asc(task::Column::OrderIndex)
            .all(&self.conn)
            .await?;

        let mut display_tasks = Vec::new();
        for task in tasks {
            display_tasks.push(self.task_to_display(task).await?);
        }
        Ok(display_tasks)
    }

    /// Get all tasks with a due date before today
    pub async fn get_overdue_tasks(&self) -> Result<Vec<task::Model>> {
        let today = chrono::Local::now().date_naive().to_string();

        let tasks = task::Entity::find()
            .filter(task::Column::IsDeleted.eq(false))
            .filter(task::Column::IsCompleted.eq(false))
            .filter(task::Column::DueDate.is_not_null())
            .filter(task::Column::DueDate.lt(&today))
            .order_by_asc(task::Column::DueDate)
            .all(&self.conn)
            .await?;

        Ok(tasks)
    }

    /// Get all tasks due on a specific date
    pub async fn get_tasks_due_on(&self, date: &str) -> Result<Vec<task::Model>> {
        let tasks = task::Entity::find()
            .filter(task::Column::IsDeleted.eq(false))
            .filter(task::Column::IsCompleted.eq(false))
            .filter(task::Column::DueDate.eq(date))
            .order_by_asc(task::Column::OrderIndex)
            .all(&self.conn)
            .await?;

        Ok(tasks)
    }

    /// Get all tasks due between two dates
    pub async fn get_tasks_due_between(&self, start_date: &str, end_date: &str) -> Result<Vec<task::Model>> {
        let tasks = task::Entity::find()
            .filter(task::Column::IsDeleted.eq(false))
            .filter(task::Column::IsCompleted.eq(false))
            .filter(task::Column::DueDate.gte(start_date))
            .filter(task::Column::DueDate.lt(end_date))
            .order_by_asc(task::Column::DueDate)
            .all(&self.conn)
            .await?;

        Ok(tasks)
    }
}
