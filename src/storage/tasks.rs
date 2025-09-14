use anyhow::Result;
use sqlx::Row;

use super::db::LocalStorage;
use crate::todoist::{Task, TaskDisplay};

/// Local task representation with sync metadata
#[derive(Debug, Clone)]
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
            "ORDER BY t.priority DESC, t.order_index ASC"
        } else {
            order_clause
        };

        let query = format!(
            r"
            SELECT
                t.id, t.content, t.project_id, t.section_id, t.parent_id, t.priority,
                t.due_date, t.due_datetime, t.is_recurring, t.deadline, t.duration, t.description,
                GROUP_CONCAT(l.id || ':' || l.name || ':' || l.color, '|') as labels_data
            FROM tasks t
            LEFT JOIN task_labels tl ON t.id = tl.task_id
            LEFT JOIN labels l ON tl.label_id = l.id
            {}
            GROUP BY t.id, t.content, t.project_id, t.section_id, t.parent_id, t.priority,
                     t.due_date, t.due_datetime, t.is_recurring, t.deadline, t.duration, t.description
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
                }
            })
            .collect();

        Ok(tasks)
    }

    /// Store task-label relationships in junction table
    async fn store_task_labels(&self, task_id: &str, label_names: &[String]) -> Result<()> {
        // First, get label IDs from names
        if label_names.is_empty() {
            return Ok(());
        }

        let placeholders = label_names.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("SELECT id, name FROM labels WHERE name IN ({placeholders})");

        let mut query_builder = sqlx::query(&query);
        for name in label_names {
            query_builder = query_builder.bind(name);
        }

        let label_rows = query_builder.fetch_all(&self.pool).await?;

        // Insert task-label relationships
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
            INSERT OR REPLACE INTO tasks (id, content, project_id, section_id, parent_id, priority, order_index, due_date, due_datetime, is_recurring, deadline, duration, description)
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

    /// Get tasks due today and overdue tasks from local storage
    pub async fn get_tasks_for_today(&self) -> Result<Vec<TaskDisplay>> {
        // Get current date in YYYY-MM-DD format
        let current_date: String = sqlx::query_scalar("SELECT date('now')").fetch_one(&self.pool).await?;

        self.get_tasks_with_labels_joined(
            "WHERE t.due_date IS NOT NULL AND t.due_date <= ?",
            r"ORDER BY
                CASE
                  WHEN t.due_date < ? THEN 0  -- Overdue first
                  ELSE 1  -- Today second
                END,
                t.priority DESC,
                t.order_index ASC",
            &[&current_date, &current_date],
        )
        .await
    }

    /// Get tasks due tomorrow from local storage
    pub async fn get_tasks_for_tomorrow(&self) -> Result<Vec<TaskDisplay>> {
        // Get tomorrow's date in YYYY-MM-DD format
        let tomorrow_date: String = sqlx::query_scalar("SELECT date('now', '+1 day')").fetch_one(&self.pool).await?;

        self.get_tasks_with_labels_joined("WHERE t.due_date IS NOT NULL AND t.due_date = ?", "", &[&tomorrow_date])
            .await
    }

    /// Get tasks for upcoming from local storage (overdue + next 3 months)
    pub async fn get_tasks_for_upcoming(&self) -> Result<Vec<TaskDisplay>> {
        self.get_tasks_with_labels_joined(
            "WHERE t.due_date IS NOT NULL AND t.due_date <= date('now', '+3 months')",
            r"ORDER BY
                CASE
                  WHEN t.due_date < date('now') THEN 0      -- Overdue tasks first
                  WHEN t.due_date = date('now') THEN 1      -- Today's tasks second
                  ELSE 2                                    -- Future tasks third
                END,
                t.due_date ASC,                            -- Then chronological order
                t.priority DESC,                           -- Then priority (high to low)
                t.order_index ASC",
            &[],
        )
        .await
    }

    /// Get a single task by ID from local storage
    pub async fn get_task_by_id(&self, task_id: &str) -> Result<Option<TaskDisplay>> {
        let tasks = self.get_tasks_with_labels_joined("WHERE t.id = ?", "", &[task_id]).await?;
        Ok(tasks.into_iter().next())
    }

    /// Delete a task and its subtasks from local storage
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
