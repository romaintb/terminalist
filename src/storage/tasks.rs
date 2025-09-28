use anyhow::Result;
use sqlx::Row;

use super::db::LocalStorage;
use crate::todoist::{Task, TaskDisplay};

/// Local task representation with sync metadata
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LocalTask {
    pub id: String,
    pub content: String,
    pub description: Option<String>,
    pub project_id: String,
    pub section_id: Option<String>,
    pub parent_id: Option<String>,
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
            id: task.id,
            content: task.content,
            project_id: task.project_id,
            section_id: task.section_id,
            parent_id: task.parent_id,
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
                t.id, t.content, t.project_id, t.section_id, t.parent_id, t.priority,
                t.due_date, t.due_datetime, t.is_recurring, t.deadline, t.duration, t.description,
                t.is_completed, t.is_deleted,
                GROUP_CONCAT(l.id || ':' || l.name || ':' || l.color, '|') as labels_data
            FROM tasks t
            LEFT JOIN task_labels tl ON t.id = tl.task_id
            LEFT JOIN labels l ON tl.label_id = l.id
            {}
            GROUP BY t.id, t.content, t.project_id, t.section_id, t.parent_id, t.priority,
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
                                        id: parts[0].to_string(),
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
                    id: row.get("id"),
                    content: row.get("content"),
                    project_id: row.get("project_id"),
                    section_id: row.get("section_id"),
                    parent_id: row.get("parent_id"),
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
    async fn store_task_labels(&self, task_id: &str, label_names: &[String]) -> Result<()> {
        // Skip if no labels
        if label_names.is_empty() {
            return Ok(());
        }

        // Get existing label IDs from names
        let placeholders = label_names.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("SELECT id, name FROM labels WHERE name IN ({placeholders})");

        let mut query_builder = sqlx::query(&query);
        for name in label_names {
            query_builder = query_builder.bind(name);
        }

        let label_rows = query_builder.fetch_all(&self.pool).await?;

        // Insert task-label relationships for found labels
        for row in label_rows {
            let label_id: String = row.get("id");
            sqlx::query("INSERT OR IGNORE INTO task_labels (task_id, label_id) VALUES (?, ?)")
                .bind(task_id)
                .bind(&label_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Remove all label relationships for a task
    async fn clear_task_labels(&self, task_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM task_labels WHERE task_id = ?")
            .bind(task_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Store tasks in local database
    pub async fn store_tasks(&self, tasks: Vec<Task>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Clear existing tasks and their label relationships
        sqlx::query("DELETE FROM task_labels").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM tasks").execute(&mut *tx).await?;

        // Collect task info for label processing
        let mut task_labels: Vec<(String, Vec<String>)> = Vec::new();

        // Insert new tasks
        for task in tasks {
            let label_names = task.labels.clone();
            let local_task: LocalTask = task.into();
            task_labels.push((local_task.id.clone(), label_names));

            sqlx::query(
                r"
                INSERT INTO tasks (id, content, project_id, section_id, parent_id, priority, order_index, due_date, due_datetime, is_recurring, deadline, duration, description)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ",
            )
            .bind(&local_task.id)
            .bind(&local_task.content)
            .bind(&local_task.project_id)
            .bind(&local_task.section_id)
            .bind(&local_task.parent_id)
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
        }

        tx.commit().await?;

        // Store label relationships after transaction commits
        for (task_id, label_names) in task_labels {
            self.store_task_labels(&task_id, &label_names).await?;
        }
        Ok(())
    }

    /// Store a single task in the database (for immediate insertion after API calls)
    pub async fn store_single_task(&self, task: Task) -> Result<()> {
        let label_names = task.labels.clone();
        let local_task: LocalTask = task.into();

        // Use transaction for atomic operation
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r"
            INSERT OR REPLACE INTO tasks (id, content, project_id, section_id, parent_id, priority, order_index, due_date, due_datetime, is_recurring, deadline, duration, description, is_completed, is_deleted)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&local_task.id)
        .bind(&local_task.content)
        .bind(&local_task.project_id)
        .bind(&local_task.section_id)
        .bind(&local_task.parent_id)
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
        self.clear_task_labels(&local_task.id).await?;
        self.store_task_labels(&local_task.id, &label_names).await?;
        Ok(())
    }

    /// Get tasks for a specific project from local storage
    pub async fn get_tasks_for_project(&self, project_id: &str) -> Result<Vec<TaskDisplay>> {
        self.get_tasks_with_labels_joined("WHERE t.project_id = ?", "", &[project_id])
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

    /// Get a single task by ID from local storage
    pub async fn get_task_by_id(&self, task_id: &str) -> Result<Option<TaskDisplay>> {
        let tasks = self.get_tasks_with_labels_joined("WHERE t.id = ?", "", &[task_id]).await?;
        Ok(tasks.into_iter().next())
    }

    /// Mark task as completed (soft completion)
    pub async fn mark_task_completed(&self, task_id: &str) -> Result<()> {
        sqlx::query("UPDATE tasks SET is_completed = 1 WHERE id = ?")
            .bind(task_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Mark task as deleted (soft deletion)
    pub async fn mark_task_deleted(&self, task_id: &str) -> Result<()> {
        sqlx::query("UPDATE tasks SET is_deleted = 1 WHERE id = ?")
            .bind(task_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Restore a soft-deleted task
    pub async fn restore_task(&self, task_id: &str) -> Result<()> {
        sqlx::query("UPDATE tasks SET is_deleted = 0, is_completed = 0 WHERE id = ?")
            .bind(task_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete a task and its subtasks from local storage (hard delete)
    /// Thanks to CASCADE DELETE, subtasks are automatically removed
    pub async fn delete_task(&self, task_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(task_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update a task's due date in local storage
    pub async fn update_task_due_date(&self, task_id: &str, due_date: Option<&str>) -> Result<()> {
        sqlx::query("UPDATE tasks SET due_date = ? WHERE id = ?")
            .bind(due_date)
            .bind(task_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Update a task's priority in local storage
    pub async fn update_task_priority(&self, task_id: &str, priority: i32) -> Result<()> {
        sqlx::query("UPDATE tasks SET priority = ? WHERE id = ?")
            .bind(priority)
            .bind(task_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
