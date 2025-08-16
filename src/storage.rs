use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, Row};

use crate::todoist::{Label, Project, ProjectDisplay, Task, TaskDisplay};

/// Local project representation with sync metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalProject {
    pub id: String,
    pub name: String,
    pub color: String,
    pub is_favorite: bool,
    pub is_inbox_project: bool,
    pub order_index: i32,
    pub parent_id: Option<String>,
    pub last_synced: DateTime<Utc>,
}

/// Local task representation with sync metadata
#[derive(Debug, Clone)]
pub struct LocalTask {
    pub id: String,
    pub content: String,
    pub description: Option<String>,
    pub project_id: String,
    pub is_completed: bool,
    pub is_deleted: bool,
    pub priority: i32,
    pub order_index: i32,
    pub due_date: Option<String>,
    pub due_datetime: Option<String>,
    pub is_recurring: bool,
    pub deadline: Option<String>,
    pub duration: Option<String>,
    pub labels: String,
    pub last_synced: String,
}

/// Local label representation with sync metadata
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LocalLabel {
    pub id: String,
    pub name: String,
    pub color: String,
    pub order_index: i32,
    pub is_favorite: bool,
    pub last_synced: DateTime<Utc>,
}

impl From<LocalProject> for ProjectDisplay {
    fn from(local: LocalProject) -> Self {
        Self {
            id: local.id,
            name: local.name,
            color: local.color,
            is_favorite: local.is_favorite,
            parent_id: local.parent_id,
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
            is_completed: local.is_completed,
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

impl From<Project> for LocalProject {
    fn from(project: Project) -> Self {
        Self {
            id: project.id,
            name: project.name,
            color: project.color,
            is_favorite: project.is_favorite,
            is_inbox_project: project.is_inbox_project,
            order_index: project.order,
            parent_id: project.parent_id,
            last_synced: Utc::now(),
        }
    }
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
            is_completed: task.is_completed,
            is_deleted: false, // New tasks are not deleted
            priority: task.priority,
            order_index: task.order,
            due_date: task.due.as_ref().map(|d| d.string.clone()),
            due_datetime: task.due.as_ref().and_then(|d| d.datetime.clone()),
            is_recurring: task.due.as_ref().map(|d| d.is_recurring).unwrap_or(false),
            deadline: task.deadline.map(|d| d.date),
            duration: duration_string,
            labels: serde_json::to_string(&task.labels).unwrap_or_default(),
            description: Some(task.description),
            last_synced: Utc::now().to_rfc3339(),
        }
    }
}

impl From<Label> for LocalLabel {
    fn from(label: Label) -> Self {
        Self {
            id: label.id,
            name: label.name,
            color: label.color,
            order_index: label.order,
            is_favorite: label.is_favorite,
            last_synced: Utc::now(),
        }
    }
}

/// Local storage manager for Todoist data
#[derive(Clone)]
pub struct LocalStorage {
    pool: SqlitePool,
}

impl LocalStorage {
    /// Initialize the local storage with `SQLite` database
    pub async fn new() -> Result<Self> {
        // Use in-memory SQLite database for ephemeral storage
        let database_url = "sqlite::memory:".to_string();
        let pool = SqlitePool::connect(&database_url).await?;
        let storage = LocalStorage { pool };
        storage.init_schema().await?;
        storage.run_migrations().await?;
        Ok(storage)
    }

    /// Initialize database schema
    async fn init_schema(&self) -> Result<()> {
        // Create projects table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT,
                is_favorite BOOLEAN NOT NULL DEFAULT 0,
                is_inbox_project BOOLEAN NOT NULL DEFAULT 0,
                order_index INTEGER NOT NULL DEFAULT 0,
                parent_id TEXT,
                last_synced TEXT NOT NULL
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create tasks table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                description TEXT,
                project_id TEXT NOT NULL,
                is_completed BOOLEAN NOT NULL DEFAULT 0,
                is_deleted BOOLEAN NOT NULL DEFAULT 0,
                priority INTEGER NOT NULL DEFAULT 1,
                order_index INTEGER NOT NULL DEFAULT 0,
                due_date TEXT,
                due_datetime TEXT,
                is_recurring BOOLEAN NOT NULL DEFAULT 0,
                deadline TEXT,
                duration TEXT,
                labels TEXT NOT NULL DEFAULT '',
                last_synced TEXT NOT NULL
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create labels table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS labels (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT NOT NULL,
                order_index INTEGER NOT NULL DEFAULT 0,
                is_favorite BOOLEAN NOT NULL DEFAULT 0,
                last_synced TEXT NOT NULL
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create sync metadata table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS sync_metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Store projects in local database
    pub async fn store_projects(&self, projects: Vec<Project>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Clear existing projects
        sqlx::query("DELETE FROM projects")
            .execute(&mut *tx)
            .await?;

        // Insert new projects
        for project in projects.iter() {
            let local_project: LocalProject = project.clone().into();
            sqlx::query(
                r"
                INSERT INTO projects (id, name, color, is_favorite, is_inbox_project, order_index, parent_id, last_synced)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                ",
            )
            .bind(&local_project.id)
            .bind(&local_project.name)
            .bind(&local_project.color)
            .bind(local_project.is_favorite)
            .bind(local_project.is_inbox_project)
            .bind(local_project.order_index)
            .bind(&local_project.parent_id)
            .bind(local_project.last_synced)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.update_sync_timestamp("projects").await?;
        Ok(())
    }

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
                INSERT INTO tasks (id, content, project_id, is_completed, is_deleted, priority, order_index, due_date, due_datetime, is_recurring, deadline, duration, labels, description, last_synced)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ",
            )
            .bind(&local_task.id)
            .bind(&local_task.content)
            .bind(&local_task.project_id)
            .bind(local_task.is_completed)
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
            .bind(local_task.last_synced)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.update_sync_timestamp("tasks").await?;
        Ok(())
    }

    /// Store labels in local database
    pub async fn store_labels(&self, labels: Vec<Label>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Clear existing labels
        sqlx::query("DELETE FROM labels").execute(&mut *tx).await?;

        // Insert new labels
        for label in labels {
            let local_label: LocalLabel = label.into();
            sqlx::query(
                r"
                INSERT INTO labels (id, name, color, order_index, is_favorite, last_synced)
                VALUES (?, ?, ?, ?, ?, ?)
                ",
            )
            .bind(&local_label.id)
            .bind(&local_label.name)
            .bind(&local_label.color)
            .bind(local_label.order_index)
            .bind(local_label.is_favorite)
            .bind(local_label.last_synced)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.update_sync_timestamp("labels").await?;
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

    /// Get all labels from local storage
    pub async fn get_all_labels(&self) -> Result<Vec<LocalLabel>> {
        let rows = sqlx::query(
            r"
            SELECT id, name, color, order_index, is_favorite, last_synced
            FROM labels 
            ORDER BY order_index ASC, name ASC
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        let labels = rows
            .into_iter()
            .map(|row| LocalLabel {
                id: row.get("id"),
                name: row.get("name"),
                color: row.get("color"),
                order_index: row.get("order_index"),
                is_favorite: row.get("is_favorite"),
                last_synced: row.get("last_synced"),
            })
            .collect();

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

    /// Delete a project and all its tasks
    pub async fn delete_project(&self, project_id: &str) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete tasks first, then the project
        sqlx::query("DELETE FROM tasks WHERE project_id = ?")
            .bind(project_id)
            .execute(&mut *tx)
            .await?;

        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(project_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Get all projects from local storage
    pub async fn get_projects(&self) -> Result<Vec<ProjectDisplay>> {
        let rows =
            sqlx::query("SELECT id, name, color, is_favorite, parent_id FROM projects ORDER BY order_index, name")
                .fetch_all(&self.pool)
                .await?;

        let projects = rows
            .into_iter()
            .map(|row| ProjectDisplay {
                id: row.get("id"),
                name: row.get("name"),
                color: row.get("color"),
                is_favorite: row.get("is_favorite"),
                parent_id: row.get("parent_id"),
            })
            .collect();

        Ok(projects)
    }

    /// Get tasks for a specific project from local storage
    pub async fn get_tasks_for_project(&self, project_id: &str) -> Result<Vec<TaskDisplay>> {
        let rows = sqlx::query(
            r"
            SELECT id, content, project_id, is_completed, is_deleted, priority, due_date, due_datetime, is_recurring, deadline, duration, labels, description
            FROM tasks 
            WHERE project_id = ? 
            ORDER BY is_completed ASC, priority DESC, order_index ASC
            ",
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        let mut tasks = rows
            .into_iter()
            .map(|row| {
                // Parse labels from JSON string
                let label_names: Vec<String> =
                    serde_json::from_str(&row.get::<String, _>("labels")).unwrap_or_default();

                // Convert label names to LabelDisplay objects (colors will be filled in later)
                let labels = label_names
                    .into_iter()
                    .map(|name| crate::todoist::LabelDisplay {
                        id: name.clone(), // Use name as ID for now
                        name,
                        color: "blue".to_string(), // Default color, will be updated from storage
                    })
                    .collect();

                TaskDisplay {
                    id: row.get("id"),
                    content: row.get("content"),
                    project_id: row.get("project_id"),
                    is_completed: row.get("is_completed"),
                    is_deleted: row.get("is_deleted"),
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
            SELECT id, content, project_id, is_completed, is_deleted, priority, due_date, due_datetime, is_recurring, deadline, duration, labels, description
            FROM tasks 
            ORDER BY is_completed ASC, priority DESC, order_index ASC
            ",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut tasks = rows
            .into_iter()
            .map(|row| {
                // Parse labels from JSON string
                let label_names: Vec<String> =
                    serde_json::from_str(&row.get::<String, _>("labels")).unwrap_or_default();

                // Convert label names to LabelDisplay objects (colors will be filled in later)
                let labels = label_names
                    .into_iter()
                    .map(|name| crate::todoist::LabelDisplay {
                        id: name.clone(), // Use name as ID for now
                        name,
                        color: "blue".to_string(), // Default color, will be updated from storage
                    })
                    .collect();

                TaskDisplay {
                    id: row.get("id"),
                    content: row.get("content"),
                    project_id: row.get("project_id"),
                    is_completed: row.get("is_completed"),
                    is_deleted: row.get("is_deleted"),
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
            .collect::<Vec<TaskDisplay>>();

        // Update label colors for all tasks
        for task in &mut tasks {
            self.update_task_labels(task).await?;
        }

        Ok(tasks)
    }

    /// Check if we have any local data
    pub async fn has_data(&self) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
            .fetch_one(&self.pool)
            .await?;
        Ok(count > 0)
    }

    /// Get last sync timestamp for a data type
    pub async fn get_last_sync(&self, data_type: &str) -> Result<Option<DateTime<Utc>>> {
        let row = sqlx::query("SELECT value FROM sync_metadata WHERE key = ?")
            .bind(format!("last_sync_{data_type}"))
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            let timestamp_str: String = row.get("value");
            Ok(Some(timestamp_str.parse()?))
        } else {
            Ok(None)
        }
    }

    /// Update sync timestamp for a data type
    async fn update_sync_timestamp(&self, data_type: &str) -> Result<()> {
        let now = Utc::now();
        sqlx::query(
            r"
            INSERT OR REPLACE INTO sync_metadata (key, value, updated_at)
            VALUES (?, ?, ?)
            ",
        )
        .bind(format!("last_sync_{data_type}"))
        .bind(now.to_rfc3339())
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark a task as completed in local storage
    pub async fn mark_task_completed(&self, task_id: &str) -> Result<()> {
        sqlx::query("UPDATE tasks SET is_completed = true, last_synced = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(task_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Mark a task as incomplete in local storage (reopen)
    pub async fn mark_task_incomplete(&self, task_id: &str) -> Result<()> {
        sqlx::query("UPDATE tasks SET is_completed = false, last_synced = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(task_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Mark a task as deleted in local storage
    pub async fn delete_task(&self, task_id: &str) -> Result<()> {
        sqlx::query("UPDATE tasks SET is_deleted = true, last_synced = ? WHERE id = ?")
            .bind(Utc::now())
            .bind(task_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Clear all local data (useful for testing or reset)
    pub async fn clear_all_data(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM tasks").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM projects")
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM sync_metadata")
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn run_migrations(&self) -> Result<()> {
        // Check if parent_id column exists in projects table
        let has_parent_id = sqlx::query_scalar::<_, Option<String>>(
            "SELECT name FROM pragma_table_info('projects') WHERE name = 'parent_id'",
        )
        .fetch_optional(&self.pool)
        .await?
        .is_some();

        if !has_parent_id {
            sqlx::query("ALTER TABLE projects ADD COLUMN parent_id TEXT")
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }
}

// Unit tests will be recreated later for proper testing
