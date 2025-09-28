use anyhow::Result;
use sqlx::Row;

use super::db::LocalStorage;
use crate::todoist::{Task, TaskDisplay};

/// Local task representation with sync metadata
#[derive(Debug, Clone)]
pub struct LocalTask {
    pub uuid: String,
    pub external_id: String,
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

impl LocalTask {
    /// Convert from API Task to LocalTask with UUID generation
    pub fn from_task_with_context(
        task: Task,
        project_uuid: String,
        section_uuid: Option<String>,
        parent_uuid: Option<String>,
    ) -> Self {
        let duration_string = task.duration.map(|d| match d.unit.as_str() {
            "minute" => format!("{}m", d.amount),
            "hour" => format!("{}h", d.amount),
            "day" => format!("{}d", d.amount),
            _ => format!("{} {}", d.amount, d.unit),
        });

        Self {
            uuid: LocalStorage::generate_uuid(),
            external_id: task.id,
            content: task.content,
            project_uuid,
            section_uuid,
            parent_uuid,
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

impl From<LocalTask> for TaskDisplay {
    fn from(local: LocalTask) -> Self {
        Self {
            uuid: local.uuid,
            content: local.content,
            project_id: local.project_uuid,
            section_id: local.section_uuid,
            parent_uuid: local.parent_uuid,
            priority: local.priority,
            due: local.due_date,
            due_datetime: local.due_datetime,
            is_recurring: local.is_recurring,
            deadline: local.deadline,
            duration: local.duration,
            labels: vec![], // Labels are fetched separately
            description: local.description.unwrap_or_default(),
            is_completed: local.is_completed,
            is_deleted: local.is_deleted,
        }
    }
}

impl LocalStorage {
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
            "ORDER BY t.is_deleted ASC, t.is_completed ASC, t.priority DESC, t.order_index ASC"
        } else {
            order_clause
        };

        let query = format!(
            r"
            SELECT
                t.uuid, t.content, t.project_uuid, t.section_uuid, t.parent_uuid, t.priority,
                t.due_date, t.due_datetime, t.is_recurring, t.deadline, t.duration, t.description,
                t.is_completed, t.is_deleted,
                GROUP_CONCAT(l.uuid || ':' || l.name || ':' || l.color, '|') as labels_data
            FROM tasks t
            LEFT JOIN task_labels tl ON t.uuid = tl.task_uuid
            LEFT JOIN labels l ON tl.label_uuid = l.uuid
            {}
            GROUP BY t.uuid, t.content, t.project_uuid, t.section_uuid, t.parent_uuid, t.priority,
                     t.due_date, t.due_datetime, t.is_recurring, t.deadline, t.duration, t.description,
                     t.is_completed, t.is_deleted
            {}
            ",
            where_part, order_part
        );

        let mut query_builder = sqlx::query(&query);
        for param in params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        let tasks = rows
            .into_iter()
            .map(|row| {
                // Parse the concatenated labels data
                let labels_data: Option<String> = row.get("labels_data");

                let labels = if let Some(data) = labels_data {
                    if data.is_empty() {
                        Vec::new()
                    } else {
                        data.split('|')
                            .filter_map(|label_str| {
                                let parts: Vec<&str> = label_str.split(':').collect();
                                if parts.len() == 3 {
                                    Some(crate::todoist::LabelDisplay {
                                        uuid: parts[0].to_string(),
                                        name: parts[1].to_string(),
                                        color: parts[2].to_string(),
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect()
                    }
                } else {
                    Vec::new()
                };

                TaskDisplay {
                    uuid: row.get("uuid"),
                    content: row.get("content"),
                    project_id: row.get("project_uuid"),
                    section_id: row.get("section_uuid"),
                    parent_uuid: row.get("parent_uuid"),
                    priority: row.get("priority"),
                    due: row.get("due_date"),
                    due_datetime: row.get("due_datetime"),
                    is_recurring: row.get("is_recurring"),
                    deadline: row.get("deadline"),
                    duration: row.get("duration"),
                    labels,
                    description: row.get("description"),
                    is_completed: row.get("is_completed"),
                    is_deleted: row.get("is_deleted"),
                }
            })
            .collect();

        Ok(tasks)
    }

    /// Store task-label relationships in junction table
    async fn store_task_labels(&self, task_uuid: &str, label_names: &[String], backend_id: &str) -> Result<()> {
        // Skip if no labels
        if label_names.is_empty() {
            return Ok(());
        }

        // Get existing label UUIDs from names for this backend
        let placeholders = label_names.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("SELECT uuid, name FROM labels WHERE name IN ({placeholders}) AND backend_id = ?");

        let mut query_builder = sqlx::query(&query);
        for name in label_names {
            query_builder = query_builder.bind(name);
        }
        query_builder = query_builder.bind(backend_id);

        let label_rows = query_builder.fetch_all(&self.pool).await?;

        // Insert task-label relationships for found labels
        for row in label_rows {
            let label_uuid: String = row.get("uuid");
            sqlx::query("INSERT OR IGNORE INTO task_labels (task_uuid, label_uuid) VALUES (?, ?)")
                .bind(task_uuid)
                .bind(&label_uuid)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Store tasks for a specific backend
    pub async fn store_tasks_for_backend(
        &self,
        backend_id: &str,
        tasks: Vec<Task>,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Clear existing tasks for this backend only (through project relationship)
        sqlx::query(
            r"
            DELETE FROM task_labels
            WHERE task_uuid IN (
                SELECT t.uuid FROM tasks t
                JOIN projects p ON t.project_uuid = p.uuid
                WHERE p.backend_id = ?
            )
            ",
        )
        .bind(backend_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r"
            DELETE FROM tasks
            WHERE project_uuid IN (
                SELECT uuid FROM projects WHERE backend_id = ?
            )
            ",
        )
        .bind(backend_id)
        .execute(&mut *tx)
        .await?;

        // Collect task info for label processing
        let mut task_labels: Vec<(String, Vec<String>)> = Vec::new();

        // Insert new tasks with proper project UUID lookup
        for task in tasks {
            let label_names = task.labels.clone();

            // Find the project UUID for this task by matching project external_id
            let project_uuid = sqlx::query_scalar::<_, String>(
                "SELECT uuid FROM projects WHERE backend_id = ? AND external_id = ?"
            )
            .bind(backend_id)
            .bind(&task.project_id)
            .fetch_optional(&mut *tx)
            .await?;

            // Skip task if project is not found - this ensures referential integrity
            let project_uuid = match project_uuid {
                Some(uuid) => uuid,
                None => {
                    log::warn!("Skipping task '{}' because project '{}' not found for backend '{}'",
                              task.content, task.project_id, backend_id);
                    continue;
                }
            };

            let local_task = LocalTask::from_task_with_context(
                task,
                project_uuid.clone(),
                None,           // No section UUID lookup for now
                None,           // No parent UUID lookup for now
            );
            task_labels.push((local_task.uuid.clone(), label_names));

            sqlx::query(
                r"
                INSERT OR REPLACE INTO tasks (uuid, external_id, content, project_uuid, section_uuid, parent_uuid, priority, order_index, due_date, due_datetime, is_recurring, deadline, duration, description, is_completed, is_deleted)
                VALUES (?, ?, ?, ?, NULL, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ",
            )
            .bind(&local_task.uuid)
            .bind(&local_task.external_id)
            .bind(&local_task.content)
            .bind(&project_uuid)
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
        }

        tx.commit().await?;

        // Store label relationships after transaction commits
        for (task_uuid, label_names) in task_labels {
            self.store_task_labels(&task_uuid, &label_names, backend_id).await?;
        }
        Ok(())
    }

    /// Store tasks in local database (legacy method - clears all backends)
    pub async fn store_tasks(&self, tasks: Vec<Task>) -> Result<()> {
        // For backward compatibility, assume single "todoist" backend
        self.store_tasks_for_backend("todoist", tasks).await
    }

    /// Store a single task for a specific backend
    pub async fn store_single_task_for_backend(
        &self,
        backend_id: &str,
        task: Task,
    ) -> Result<()> {
        let label_names = task.labels.clone();

        // Use INSERT OR REPLACE to handle both new and existing tasks
        let local_task = LocalTask::from_task_with_context(
            task,
            "".to_string(), // Empty project UUID for now
            None,           // No section UUID
            None,           // No parent UUID
        );

        sqlx::query(
            r"
            INSERT OR REPLACE INTO tasks (uuid, external_id, content, project_uuid, section_uuid, parent_uuid, priority, order_index, due_date, due_datetime, is_recurring, deadline, duration, description, is_completed, is_deleted)
            VALUES (?, ?, ?, NULL, NULL, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&local_task.uuid)
        .bind(&local_task.external_id)
        .bind(&local_task.content)
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
        .execute(&self.pool)
        .await?;

        // Store label relationships
        self.store_task_labels(&local_task.uuid, &label_names, backend_id).await?;

        Ok(())
    }

    /// Store a single task in the database (legacy method - assumes "todoist" backend)
    pub async fn store_single_task(&self, task: Task) -> Result<()> {
        self.store_single_task_for_backend("todoist", task).await
    }

    /// Get tasks for a specific project from local storage
    pub async fn get_tasks_for_project(&self, project_uuid: &str) -> Result<Vec<TaskDisplay>> {
        self.get_tasks_with_labels_joined("WHERE t.project_uuid = ?", "", &[project_uuid])
            .await
    }

    /// Get all tasks from local storage
    pub async fn get_all_tasks(&self) -> Result<Vec<TaskDisplay>> {
        self.get_tasks_with_labels_joined("", "", &[]).await
    }

    /// Get tasks for a specific backend
    pub async fn get_tasks_for_backend(&self, backend_id: &str) -> Result<Vec<TaskDisplay>> {
        self.get_tasks_with_labels_joined(
            r"
            WHERE t.project_uuid IN (
                SELECT uuid FROM projects WHERE backend_id = ?
            )
            ",
            "",
            &[backend_id],
        )
        .await
    }

    /// Search tasks by content using SQL LIKE (case-insensitive)
    pub async fn search_tasks(&self, query: &str) -> Result<Vec<TaskDisplay>> {
        if query.is_empty() {
            return self.get_all_tasks().await;
        }

        // Use SQL LIKE with wildcards for efficient database-level search
        let search_pattern = format!("%{}%", query);
        self.get_tasks_with_labels_joined(
            "WHERE LOWER(t.content) LIKE LOWER(?)",
            "ORDER BY t.is_deleted ASC, t.is_completed ASC, t.priority DESC, t.order_index ASC",
            &[&search_pattern],
        )
        .await
    }

    /// Get tasks due on a specific date (pure data access)
    pub async fn get_tasks_due_on(&self, date: &str) -> Result<Vec<TaskDisplay>> {
        self.get_tasks_with_labels_joined(
            "WHERE t.due_date IS NOT NULL AND t.due_date = ?",
            "ORDER BY t.is_deleted ASC, t.is_completed ASC, t.priority DESC, t.order_index ASC",
            &[date],
        )
        .await
    }

    /// Get overdue tasks (pure data access)
    pub async fn get_overdue_tasks(&self) -> Result<Vec<TaskDisplay>> {
        self.get_tasks_with_labels_joined(
            "WHERE t.due_date IS NOT NULL AND t.due_date < date('now')",
            "ORDER BY t.is_deleted ASC, t.is_completed ASC, t.due_date ASC, t.priority DESC, t.order_index ASC",
            &[],
        )
        .await
    }

    /// Get tasks due between two dates (inclusive)
    pub async fn get_tasks_due_between(&self, start_date: &str, end_date: &str) -> Result<Vec<TaskDisplay>> {
        self.get_tasks_with_labels_joined(
            "WHERE t.due_date IS NOT NULL AND t.due_date >= ? AND t.due_date <= ?",
            "ORDER BY t.is_deleted ASC, t.is_completed ASC, t.due_date ASC, t.priority DESC, t.order_index ASC",
            &[start_date, end_date],
        )
        .await
    }

    /// Get tasks with no due date
    pub async fn get_tasks_without_due_date(&self) -> Result<Vec<TaskDisplay>> {
        self.get_tasks_with_labels_joined(
            "WHERE t.due_date IS NULL",
            "ORDER BY t.is_deleted ASC, t.is_completed ASC, t.priority DESC, t.order_index ASC",
            &[],
        )
        .await
    }

    /// Get a single task by UUID from local storage
    pub async fn get_task_by_uuid(&self, task_uuid: &str) -> Result<Option<TaskDisplay>> {
        let tasks = self.get_tasks_with_labels_joined("WHERE t.uuid = ?", "", &[task_uuid]).await?;
        Ok(tasks.into_iter().next())
    }

    /// Get a single task by ID from local storage (legacy method)
    pub async fn get_task_by_id(&self, task_id: &str) -> Result<Option<TaskDisplay>> {
        self.get_task_by_uuid(task_id).await
    }

    /// Mark task as completed (soft completion)
    pub async fn mark_task_completed(&self, task_uuid: &str) -> Result<()> {
        sqlx::query("UPDATE tasks SET is_completed = 1 WHERE uuid = ?")
            .bind(task_uuid)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Mark task as deleted (soft deletion)
    pub async fn mark_task_deleted(&self, task_uuid: &str) -> Result<()> {
        sqlx::query("UPDATE tasks SET is_deleted = 1 WHERE uuid = ?")
            .bind(task_uuid)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Restore a soft-deleted task
    pub async fn restore_task(&self, task_uuid: &str) -> Result<()> {
        sqlx::query("UPDATE tasks SET is_deleted = 0, is_completed = 0 WHERE uuid = ?")
            .bind(task_uuid)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete a task and its subtasks from local storage (hard delete)
    /// Thanks to CASCADE DELETE, subtasks are automatically removed
    pub async fn delete_task(&self, task_uuid: &str) -> Result<()> {
        sqlx::query("DELETE FROM tasks WHERE uuid = ?")
            .bind(task_uuid)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update a task's due date in local storage
    pub async fn update_task_due_date(&self, task_uuid: &str, due_date: Option<&str>) -> Result<()> {
        sqlx::query("UPDATE tasks SET due_date = ? WHERE uuid = ?")
            .bind(due_date)
            .bind(task_uuid)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update a task's priority in local storage
    pub async fn update_task_priority(&self, task_uuid: &str, priority: i32) -> Result<()> {
        sqlx::query("UPDATE tasks SET priority = ? WHERE uuid = ?")
            .bind(priority)
            .bind(task_uuid)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Sync a task from a backend (maintains existing UUID if found)
    pub async fn sync_task_from_backend(
        &self,
        backend_id: &str,
        task: &todoist_api::Task,
    ) -> Result<()> {
        // Check if we already have this task for this backend
        let existing_uuid = sqlx::query_scalar::<_, String>(
            "SELECT uuid FROM tasks WHERE external_id = ?"
        )
        .bind(&task.id)
        .fetch_optional(&self.pool)
        .await?;

        let uuid = existing_uuid.unwrap_or_else(|| Self::generate_uuid());

        // Find the project UUID for this task by matching project external_id
        let project_uuid = sqlx::query_scalar::<_, String>(
            "SELECT uuid FROM projects WHERE backend_id = ? AND external_id = ?"
        )
        .bind(backend_id)
        .bind(&task.project_id)
        .fetch_optional(&self.pool)
        .await?;

        // Skip task if project is not found - this ensures referential integrity
        let project_uuid = match project_uuid {
            Some(uuid) => uuid,
            None => {
                log::warn!("Skipping task '{}' because project '{}' not found for backend '{}'",
                          task.content, task.project_id, backend_id);
                return Ok(());
            }
        };

        // Find the section UUID if section_id is provided
        let section_uuid = if let Some(section_id) = &task.section_id {
            sqlx::query_scalar::<_, String>(
                "SELECT uuid FROM sections WHERE external_id = ?"
            )
            .bind(section_id)
            .fetch_optional(&self.pool)
            .await?
        } else {
            None
        };

        // Convert duration from API format to string
        let duration_string = task.duration.as_ref().map(|d| match d.unit.as_str() {
            "minute" => format!("{}m", d.amount),
            "hour" => format!("{}h", d.amount),
            "day" => format!("{}d", d.amount),
            _ => format!("{} {}", d.amount, d.unit),
        });

        // Insert or update the task
        sqlx::query(
            r"
            INSERT OR REPLACE INTO tasks
            (uuid, external_id, content, description, project_uuid, section_uuid, parent_uuid,
             priority, due_date, due_datetime, is_recurring, deadline_date, duration,
             is_completed, is_deleted)
            VALUES (?, ?, ?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, 0)
            ",
        )
        .bind(&uuid)
        .bind(&task.id)
        .bind(&task.content)
        .bind(&task.description)
        .bind(project_uuid)
        .bind(section_uuid)
        .bind(task.priority)
        .bind(task.due.as_ref().map(|d| d.date.clone()))
        .bind(task.due.as_ref().and_then(|d| d.datetime.clone()))
        .bind(task.due.as_ref().map(|d| d.is_recurring).unwrap_or(false))
        .bind(task.deadline.as_ref().map(|d| d.date.clone()))
        .bind(duration_string)
        .bind(task.is_completed)
        .execute(&self.pool)
        .await?;

        // Handle labels separately (since they're stored as a JSON array or separate table)
        // For now, let's serialize labels as JSON in the labels column
        let labels_json = serde_json::to_string(&task.labels)?;
        sqlx::query("UPDATE tasks SET labels = ? WHERE uuid = ?")
            .bind(labels_json)
            .bind(&uuid)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
