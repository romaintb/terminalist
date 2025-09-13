use anyhow::Result;

use super::db::LocalStorage;
use super::labels::LocalLabel;
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
    pub is_deleted: bool,
    pub priority: i32,
    pub order_index: i32,
    pub due_date: Option<String>,
    pub due_datetime: Option<String>,
    pub is_recurring: bool,
    pub deadline: Option<String>,
    pub duration: Option<String>,
    pub labels: String,
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
            is_deleted: false, // New tasks are not deleted
            priority: task.priority,
            order_index: task.order,
            due_date: task.due.as_ref().map(|d| d.date.clone()),
            due_datetime: task.due.as_ref().and_then(|d| d.datetime.clone()),
            is_recurring: task.due.as_ref().is_some_and(|d| d.is_recurring),
            deadline: task.deadline.map(|d| d.date),
            duration: duration_string,
            labels: serde_json::to_string(&task.labels).unwrap_or_default(),
            description: Some(task.description),
        }
    }
}

impl From<LocalTask> for TaskDisplay {
    fn from(local: LocalTask) -> Self {
        // Parse labels from JSON string
        let label_names: Vec<String> = serde_json::from_str(&local.labels).unwrap_or_default();

        // Convert label names to LabelDisplay objects (colors will be filled in later)
        let labels = label_names
            .into_iter()
            .map(|name| crate::todoist::LabelDisplay {
                id: name.clone(), // Use name as ID for now
                name,
                color: "blue".to_string(), // Default color, will be updated from storage
            })
            .collect();

        Self {
            id: local.id,
            content: local.content,
            project_id: local.project_id,
            section_id: local.section_id,
            parent_id: local.parent_id,
            is_deleted: local.is_deleted,
            priority: local.priority,
            due: local.due_date,
            due_datetime: local.due_datetime,
            is_recurring: local.is_recurring,
            deadline: local.deadline,
            duration: local.duration,
            labels,
            description: local.description.unwrap_or_default(),
        }
    }
}

impl LocalStorage {
    /// Store tasks in local database
    pub async fn store_tasks(&self, tasks: Vec<Task>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Clear existing tasks
        sqlx::query("DELETE FROM tasks").execute(&mut *tx).await?;

        // Insert new tasks
        for task in tasks {
            let local_task: LocalTask = task.into();
            sqlx::query(
                r"
                INSERT INTO tasks (id, content, project_id, section_id, parent_id, is_deleted, priority, order_index, due_date, due_datetime, is_recurring, deadline, duration, labels, description)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ",
            )
            .bind(&local_task.id)
            .bind(&local_task.content)
            .bind(&local_task.project_id)
            .bind(&local_task.section_id)
            .bind(&local_task.parent_id)
            .bind(local_task.is_deleted)
            .bind(local_task.priority)
            .bind(local_task.order_index)
            .bind(&local_task.due_date)
            .bind(&local_task.due_datetime)
            .bind(local_task.is_recurring)
            .bind(&local_task.deadline)
            .bind(&local_task.duration)
            .bind(&local_task.labels)
            .bind(&local_task.description)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Store a single task in the database (for immediate insertion after API calls)
    pub async fn store_single_task(&self, task: Task) -> Result<()> {
        let local_task: LocalTask = task.into();

        sqlx::query(
            r"
            INSERT OR REPLACE INTO tasks (id, content, project_id, section_id, parent_id, is_deleted, priority, order_index, due_date, due_datetime, is_recurring, deadline, duration, labels, description)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(&local_task.id)
        .bind(&local_task.content)
        .bind(&local_task.project_id)
        .bind(&local_task.section_id)
        .bind(&local_task.parent_id)
        .bind(local_task.is_deleted)
        .bind(local_task.priority)
        .bind(local_task.order_index)
        .bind(&local_task.due_date)
        .bind(&local_task.due_datetime)
        .bind(local_task.is_recurring)
        .bind(&local_task.deadline)
        .bind(&local_task.duration)
        .bind(&local_task.labels)
        .bind(&local_task.description)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get labels by their IDs
    pub async fn get_labels_by_ids(&self, label_ids: &[String]) -> Result<Vec<LocalLabel>> {
        if label_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Create placeholders for the IN clause
        let placeholders = label_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!("SELECT * FROM labels WHERE id IN ({placeholders}) ORDER BY order_index");

        let mut query_builder = sqlx::query_as::<_, LocalLabel>(&query);
        for id in label_ids {
            query_builder = query_builder.bind(id);
        }

        let labels = query_builder.fetch_all(&self.pool).await?;
        Ok(labels)
    }

    /// Update task labels with proper color information
    pub async fn update_task_labels(&self, task_display: &mut TaskDisplay) -> Result<()> {
        if task_display.labels.is_empty() {
            return Ok(());
        }

        // Extract label names from the task
        let label_names: Vec<String> = task_display.labels.iter().map(|l| l.name.clone()).collect();

        // Get the actual label objects from storage
        let stored_labels = self.get_labels_by_ids(&label_names).await?;

        // Create a map of label names to colors
        let mut label_color_map = std::collections::HashMap::new();
        for label in stored_labels {
            label_color_map.insert(label.name, label.color);
        }

        // Update the task labels with proper colors
        for label_display in &mut task_display.labels {
            if let Some(color) = label_color_map.get(&label_display.name) {
                label_display.color = color.clone();
            }
        }

        Ok(())
    }

    /// Get tasks for a specific project from local storage
    pub async fn get_tasks_for_project(&self, project_id: &str) -> Result<Vec<TaskDisplay>> {
        let rows = sqlx::query(
            r"
            SELECT id, content, project_id, section_id, parent_id, is_deleted, priority, due_date, due_datetime, is_recurring, deadline, duration, labels, description
            FROM tasks
            WHERE project_id = ?
            ORDER BY priority DESC, order_index ASC
            ",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        let mut tasks = rows
            .into_iter()
            .map(|row| Self::task_display_from_row(&row))
            .collect::<Vec<TaskDisplay>>();

        // Update label colors for all tasks
        for task in &mut tasks {
            self.update_task_labels(task).await?;
        }

        Ok(tasks)
    }

    /// Get all tasks from local storage
    pub async fn get_all_tasks(&self) -> Result<Vec<TaskDisplay>> {
        let rows = sqlx::query(
            r"
            SELECT id, content, project_id, section_id, parent_id, is_deleted, priority, due_date, due_datetime, is_recurring, deadline, duration, labels, description
            FROM tasks
            ORDER BY priority DESC, order_index ASC
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut tasks = rows
            .into_iter()
            .map(|row| Self::task_display_from_row(&row))
            .collect::<Vec<TaskDisplay>>();

        // Update label colors for all tasks
        for task in &mut tasks {
            self.update_task_labels(task).await?;
        }

        Ok(tasks)
    }

    /// Get tasks due today and overdue tasks from local storage
    pub async fn get_tasks_for_today(&self) -> Result<Vec<TaskDisplay>> {
        // Get current date in YYYY-MM-DD format
        let current_date: String = sqlx::query_scalar("SELECT date('now')").fetch_one(&self.pool).await?;

        let rows = sqlx::query(
            r"
            SELECT id, content, project_id, section_id, parent_id, is_deleted, priority, due_date, due_datetime, is_recurring, deadline, duration, labels, description
            FROM tasks
            WHERE is_deleted = false
              AND due_date IS NOT NULL
              AND due_date <= ?
            ORDER BY
              CASE
                WHEN due_date < ? THEN 0  -- Overdue first
                ELSE 1  -- Today second
              END,
              priority DESC,
              order_index ASC
            ",
        )
        .bind(&current_date)
        .bind(&current_date)
        .fetch_all(&self.pool)
        .await?;

        let mut tasks = rows
            .into_iter()
            .map(|row| Self::task_display_from_row(&row))
            .collect::<Vec<TaskDisplay>>();

        // Update label colors for all tasks
        for task in &mut tasks {
            self.update_task_labels(task).await?;
        }

        Ok(tasks)
    }

    /// Get tasks due tomorrow from local storage
    pub async fn get_tasks_for_tomorrow(&self) -> Result<Vec<TaskDisplay>> {
        // Get tomorrow's date in YYYY-MM-DD format
        let tomorrow_date: String = sqlx::query_scalar("SELECT date('now', '+1 day')").fetch_one(&self.pool).await?;

        let rows = sqlx::query(
            r"
            SELECT id, content, project_id, section_id, parent_id, is_deleted, priority, due_date, due_datetime, is_recurring, deadline, duration, labels, description
            FROM tasks
            WHERE is_deleted = false
              AND due_date IS NOT NULL
              AND due_date = ?
            ORDER BY
              priority DESC,
              order_index ASC
            ",
        )
        .bind(&tomorrow_date)
        .fetch_all(&self.pool)
        .await?;

        let mut tasks = rows
            .into_iter()
            .map(|row| Self::task_display_from_row(&row))
            .collect::<Vec<TaskDisplay>>();

        // Update label colors for all tasks
        for task in &mut tasks {
            self.update_task_labels(task).await?;
        }

        Ok(tasks)
    }

    /// Get tasks for upcoming from local storage (overdue + next 3 months)
    pub async fn get_tasks_for_upcoming(&self) -> Result<Vec<TaskDisplay>> {
        let rows = sqlx::query(
            r"
            SELECT *
            FROM tasks
            WHERE is_deleted = false
              AND due_date IS NOT NULL
              AND due_date <= date('now', '+3 months')
            ORDER BY
              CASE
                WHEN due_date < date('now') THEN 0      -- Overdue tasks first
                WHEN due_date = date('now') THEN 1      -- Today's tasks second
                ELSE 2                                  -- Future tasks third
              END,
              due_date ASC,                            -- Then chronological order
              priority DESC,                           -- Then priority (high to low)
              order_index ASC                          -- Finally by user's manual order
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut tasks = rows
            .into_iter()
            .map(|row| Self::task_display_from_row(&row))
            .collect::<Vec<TaskDisplay>>();

        // Update label colors for all tasks
        for task in &mut tasks {
            self.update_task_labels(task).await?;
        }

        Ok(tasks)
    }

    /// Get a single task by ID from local storage
    pub async fn get_task_by_id(&self, task_id: &str) -> Result<Option<TaskDisplay>> {
        let row = sqlx::query(
            r"
            SELECT id, content, project_id, section_id, parent_id, is_deleted, priority, due_date, due_datetime, is_recurring, deadline, duration, labels, description
            FROM tasks
            WHERE id = ? AND is_deleted = false
            ",
        )
        .bind(task_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let mut task = Self::task_display_from_row(&row);

            // Update label colors
            self.update_task_labels(&mut task).await?;

            Ok(Some(task))
        } else {
            Ok(None)
        }
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
        sqlx::query("UPDATE tasks SET priority = ?, WHERE id = ?")
            .bind(priority)
            .bind(task_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Mark a task as deleted in local storage
    pub async fn mark_task_deleted(&self, task_id: &str) -> Result<()> {
        sqlx::query("UPDATE tasks SET is_deleted = true WHERE id = ?")
            .bind(task_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
