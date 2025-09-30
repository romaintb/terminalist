use anyhow::Result;

use super::{db::LocalStorage, labels::LocalLabel};
use crate::todoist::{LabelDisplay, Task, TaskDisplay};

/// Local task representation with sync metadata
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LocalTask {
    pub uuid: String,
    pub remote_id: String,
    pub content: String,
    pub description: Option<String>,
    pub project_uuid: String,
    pub section_uuid: Option<String>,
    pub parent_uuid: Option<String>,
    pub priority: i32,
    pub order_index: i32,
    pub due_date: Option<String>,
    pub due_datetime: Option<String>,
    pub is_recurring: bool,
    pub deadline: Option<String>,
    pub duration: Option<String>,
    pub is_completed: bool,
    pub is_deleted: bool,
}

impl From<Task> for LocalTask {
    fn from(task: Task) -> Self {
        let duration_string = task.duration.map(|d| match d.unit.as_str() {
            "minute" => format!("{}m", d.amount),
            "hour" => format!("{}h", d.amount),
            "day" => format!("{}d", d.amount),
            _ => format!("{} {}", d.amount, d.unit),
        });

        Self {
            uuid: uuid::Uuid::new_v4().to_string(),
            remote_id: task.id,
            content: task.content,
            project_uuid: String::new(), // Will be resolved at storage layer
            section_uuid: None,          // Will be resolved at storage layer
            parent_uuid: None,           // Will be resolved at storage layer
            priority: task.priority,
            order_index: task.order,
            due_date: task.due.as_ref().map(|d| d.date.clone()),
            due_datetime: task.due.as_ref().and_then(|d| d.datetime.clone()),
            is_recurring: task.due.as_ref().is_some_and(|d| d.is_recurring),
            deadline: task.deadline.map(|d| d.date),
            duration: duration_string,
            description: Some(task.description),
            is_completed: task.is_completed,
            is_deleted: false, // Tasks from API are never deleted locally
        }
    }
}

impl LocalStorage {
    /// Get labels for a specific task
    async fn get_labels_for_task(&self, task_id: &str) -> Result<Vec<LabelDisplay>> {
        let labels = sqlx::query_as::<_, LocalLabel>(
            r"
            SELECT l.uuid, l.remote_id, l.name, l.color, l.order_index, l.is_favorite
            FROM labels l
            INNER JOIN task_labels tl ON l.uuid = tl.label_uuid
            WHERE tl.task_uuid = ?
            ORDER BY l.order_index ASC
            ",
        )
        .bind(task_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|label| LabelDisplay {
            id: label.uuid,
            name: label.name,
            color: label.color,
        })
        .collect();

        Ok(labels)
    }

    async fn get_tasks_with_labels_joined(
        &self,
        where_clause: &str,
        order_clause: &str,
        params: &[&str],
    ) -> Result<Vec<TaskDisplay>> {
        let where_part = if where_clause.is_empty() { "" } else { where_clause };
        let order_part = if order_clause.is_empty() {
            // Default ordering: uncompleted first, then completed, then deleted
            // Within each group, order by priority and then by order_index
            "ORDER BY is_deleted ASC, is_completed ASC, priority DESC, order_index ASC"
        } else {
            order_clause
        };

        let query = format!(
            r"
            SELECT
                uuid, remote_id, content, project_uuid, section_uuid, parent_uuid, priority, order_index,
                due_date, due_datetime, is_recurring, deadline, duration, description,
                is_completed, is_deleted
            FROM tasks
            {}
            {}
            ",
            where_part, order_part
        );

        let mut query_builder = sqlx::query_as::<_, LocalTask>(&query);
        for param in params {
            query_builder = query_builder.bind(param);
        }

        let tasks = query_builder.fetch_all(&self.pool).await?;

        let mut task_displays = Vec::new();
        for task in tasks {
            let labels = self.get_labels_for_task(&task.uuid).await?;
            task_displays.push(TaskDisplay {
                id: task.remote_id,
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
            });
        }

        Ok(task_displays)
    }

    /// Store task-label relationships in junction table
    async fn store_task_labels(&self, task_id: &str, label_names: &[String]) -> Result<()> {
        // Skip if no labels
        if label_names.is_empty() {
            return Ok(());
        }

        #[derive(sqlx::FromRow)]
        struct LabelUuidRow {
            uuid: String,
        }

        // Get existing label UUIDs from names
        let placeholders = label_names.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("SELECT uuid FROM labels WHERE name IN ({placeholders})");

        let mut query_builder = sqlx::query_as::<_, LabelUuidRow>(&query);
        for name in label_names {
            query_builder = query_builder.bind(name);
        }

        let label_rows = query_builder.fetch_all(&self.pool).await?;

        // Insert task-label relationships for found labels
        for row in label_rows {
            sqlx::query("INSERT OR IGNORE INTO task_labels (task_uuid, label_uuid) VALUES (?, ?)")
                .bind(task_id)
                .bind(&row.uuid)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Remove all label relationships for a task
    async fn clear_task_labels(&self, task_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM task_labels WHERE task_uuid = ?")
            .bind(task_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Store tasks in local database
    pub async fn store_tasks(&self, tasks: Vec<Task>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // First pass: Upsert all tasks without parent_uuid relationships
        // This preserves existing UUIDs when remote_id matches
        let mut task_labels: Vec<(String, Vec<String>)> = Vec::new();

        for task in &tasks {
            let label_names = task.labels.clone();
            let mut local_task: LocalTask = task.clone().into();

            // Look up local project UUID from remote project_id
            if let Some(local_project_uuid) = self.find_uuid_by_remote_id(&mut tx, "projects", &task.project_id).await?
            {
                local_task.project_uuid = local_project_uuid;
            }

            // Look up local section UUID from remote section_id if present
            if let Some(remote_section_id) = &task.section_id {
                if let Some(local_section_uuid) =
                    self.find_uuid_by_remote_id(&mut tx, "sections", remote_section_id).await?
                {
                    local_task.section_uuid = Some(local_section_uuid);
                }
            }

            sqlx::query(
                r"
                INSERT INTO tasks (uuid, remote_id, content, project_uuid, section_uuid, parent_uuid, priority, order_index, due_date, due_datetime, is_recurring, deadline, duration, description)
                VALUES (?, ?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(remote_id) DO UPDATE SET
                    content = excluded.content,
                    project_uuid = excluded.project_uuid,
                    section_uuid = excluded.section_uuid,
                    parent_uuid = NULL,
                    priority = excluded.priority,
                    order_index = excluded.order_index,
                    due_date = excluded.due_date,
                    due_datetime = excluded.due_datetime,
                    is_recurring = excluded.is_recurring,
                    deadline = excluded.deadline,
                    duration = excluded.duration,
                    description = excluded.description
                ",
            )
            .bind(&local_task.uuid)
            .bind(&local_task.remote_id)
            .bind(&local_task.content)
            .bind(&local_task.project_uuid)
            .bind(&local_task.section_uuid)
            .bind(local_task.priority)
            .bind(local_task.order_index)
            .bind(&local_task.due_date)
            .bind(&local_task.due_datetime)
            .bind(local_task.is_recurring)
            .bind(&local_task.deadline)
            .bind(&local_task.duration)
            .bind(&local_task.description)
            .execute(&mut *tx)
            .await?;

            // Get the uuid of the task we just inserted/updated to use for label relationships
            if let Some(task_uuid) = self.find_uuid_by_remote_id(&mut tx, "tasks", &task.id).await? {
                task_labels.push((task_uuid, label_names));
            }
        }

        // Second pass: Update parent_uuid references to use local UUIDs
        for task in &tasks {
            if let Some(remote_parent_id) = &task.parent_id {
                if let Some(local_parent_uuid) = self.find_uuid_by_remote_id(&mut tx, "tasks", remote_parent_id).await?
                {
                    sqlx::query("UPDATE tasks SET parent_uuid = ? WHERE remote_id = ?")
                        .bind(&local_parent_uuid)
                        .bind(&task.id)
                        .execute(&mut *tx)
                        .await?;
                }
            }
        }

        tx.commit().await?;

        // Clear and recreate label relationships
        sqlx::query("DELETE FROM task_labels").execute(&self.pool).await?;
        for (task_uuid, label_names) in task_labels {
            self.store_task_labels(&task_uuid, &label_names).await?;
        }
        Ok(())
    }

    /// Store a single task in the database (for immediate insertion after API calls)
    pub async fn store_single_task(&self, task: Task) -> Result<()> {
        let label_names = task.labels.clone();
        let mut local_task: LocalTask = task.clone().into();

        // Use transaction for atomic operation
        let mut tx = self.pool.begin().await?;

        // Look up local project UUID from remote project_id
        if let Some(local_project_uuid) = self.find_uuid_by_remote_id(&mut tx, "projects", &task.project_id).await? {
            local_task.project_uuid = local_project_uuid;
        }

        // Look up local section UUID from remote section_id if present
        if let Some(remote_section_id) = &task.section_id {
            if let Some(local_section_uuid) =
                self.find_uuid_by_remote_id(&mut tx, "sections", remote_section_id).await?
            {
                local_task.section_uuid = Some(local_section_uuid);
            }
        }

        // Look up local parent UUID from remote parent_id if present
        if let Some(remote_parent_id) = &task.parent_id {
            if let Some(local_parent_uuid) = self.find_uuid_by_remote_id(&mut tx, "tasks", remote_parent_id).await? {
                local_task.parent_uuid = Some(local_parent_uuid);
            }
        }

        sqlx::query(
            r"
            INSERT OR REPLACE INTO tasks (uuid, remote_id, content, project_uuid, section_uuid, parent_uuid, priority, order_index, due_date, due_datetime, is_recurring, deadline, duration, description, is_completed, is_deleted)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&local_task.uuid)
        .bind(&local_task.remote_id)
        .bind(&local_task.content)
        .bind(&local_task.project_uuid)
        .bind(&local_task.section_uuid)
        .bind(&local_task.parent_uuid)
        .bind(local_task.priority)
        .bind(local_task.order_index)
        .bind(&local_task.due_date)
        .bind(&local_task.due_datetime)
        .bind(local_task.is_recurring)
        .bind(&local_task.deadline)
        .bind(&local_task.duration)
        .bind(&local_task.description)
        .bind(local_task.is_completed)
        .bind(local_task.is_deleted)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        // Store label relationships after transaction commits
        self.clear_task_labels(&local_task.uuid).await?;
        self.store_task_labels(&local_task.uuid, &label_names).await?;
        Ok(())
    }

    /// Get tasks for a specific project from local storage
    pub async fn get_tasks_for_project(&self, project_id: &str) -> Result<Vec<TaskDisplay>> {
        self.get_tasks_with_labels_joined("WHERE project_uuid = ?", "", &[project_id])
            .await
    }

    /// Get all tasks from local storage
    pub async fn get_all_tasks(&self) -> Result<Vec<TaskDisplay>> {
        self.get_tasks_with_labels_joined("", "", &[]).await
    }

    /// Search tasks by content using SQL LIKE (case-insensitive)
    pub async fn search_tasks(&self, query: &str) -> Result<Vec<TaskDisplay>> {
        if query.is_empty() {
            return self.get_all_tasks().await;
        }

        // Use SQL LIKE with wildcards for efficient database-level search
        let search_pattern = format!("%{}%", query);
        self.get_tasks_with_labels_joined(
            "WHERE LOWER(content) LIKE LOWER(?)",
            "ORDER BY is_deleted ASC, is_completed ASC, priority DESC, order_index ASC",
            &[&search_pattern],
        )
        .await
    }

    /// Get tasks due on a specific date (pure data access)
    pub async fn get_tasks_due_on(&self, date: &str) -> Result<Vec<TaskDisplay>> {
        self.get_tasks_with_labels_joined(
            "WHERE due_date IS NOT NULL AND due_date = ?",
            "ORDER BY is_deleted ASC, is_completed ASC, priority DESC, order_index ASC",
            &[date],
        )
        .await
    }

    /// Get overdue tasks (pure data access)
    pub async fn get_overdue_tasks(&self) -> Result<Vec<TaskDisplay>> {
        self.get_tasks_with_labels_joined(
            "WHERE due_date IS NOT NULL AND due_date < date('now')",
            "ORDER BY is_deleted ASC, is_completed ASC, due_date ASC, priority DESC, order_index ASC",
            &[],
        )
        .await
    }

    /// Get tasks due between two dates (inclusive)
    pub async fn get_tasks_due_between(&self, start_date: &str, end_date: &str) -> Result<Vec<TaskDisplay>> {
        self.get_tasks_with_labels_joined(
            "WHERE due_date IS NOT NULL AND due_date >= ? AND due_date <= ?",
            "ORDER BY is_deleted ASC, is_completed ASC, due_date ASC, priority DESC, order_index ASC",
            &[start_date, end_date],
        )
        .await
    }

    /// Get tasks with no due date
    pub async fn get_tasks_without_due_date(&self) -> Result<Vec<TaskDisplay>> {
        self.get_tasks_with_labels_joined(
            "WHERE due_date IS NULL",
            "ORDER BY is_deleted ASC, is_completed ASC, priority DESC, order_index ASC",
            &[],
        )
        .await
    }

    /// Get a single task by ID from local storage
    pub async fn get_task_by_id(&self, task_id: &str) -> Result<Option<TaskDisplay>> {
        let tasks = self.get_tasks_with_labels_joined("WHERE uuid = ?", "", &[task_id]).await?;
        Ok(tasks.into_iter().next())
    }

    /// Mark task as completed (soft completion)
    pub async fn mark_task_completed(&self, task_id: &str) -> Result<()> {
        sqlx::query("UPDATE tasks SET is_completed = 1 WHERE uuid = ?")
            .bind(task_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Mark task as deleted (soft deletion)
    pub async fn mark_task_deleted(&self, task_id: &str) -> Result<()> {
        sqlx::query("UPDATE tasks SET is_deleted = 1 WHERE uuid = ?")
            .bind(task_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Restore a soft-deleted task
    pub async fn restore_task(&self, task_id: &str) -> Result<()> {
        sqlx::query("UPDATE tasks SET is_deleted = 0, is_completed = 0 WHERE uuid = ?")
            .bind(task_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete a task and its subtasks from local storage (hard delete)
    /// Thanks to CASCADE DELETE, subtasks are automatically removed
    pub async fn delete_task(&self, task_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM tasks WHERE uuid = ?")
            .bind(task_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update a task's due date in local storage
    pub async fn update_task_due_date(&self, task_id: &str, due_date: Option<&str>) -> Result<()> {
        sqlx::query("UPDATE tasks SET due_date = ? WHERE uuid = ?")
            .bind(due_date)
            .bind(task_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update a task's priority in local storage
    pub async fn update_task_priority(&self, task_id: &str, priority: i32) -> Result<()> {
        sqlx::query("UPDATE tasks SET priority = ? WHERE uuid = ?")
            .bind(priority)
            .bind(task_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
