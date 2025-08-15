use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, Row};

use crate::todoist::{Project, ProjectDisplay, Task, TaskDisplay};

/// Local project representation with sync metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalProject {
    pub id: String,
    pub name: String,
    pub color: String,
    pub is_favorite: bool,
    pub is_inbox_project: bool,
    pub order_index: i32,
    pub last_synced: DateTime<Utc>,
}

/// Local task representation with sync metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalTask {
    pub id: String,
    pub content: String,
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
    pub labels: String, // JSON serialized array
    pub description: String,
    pub last_synced: DateTime<Utc>,
}

impl From<LocalProject> for ProjectDisplay {
    fn from(local: LocalProject) -> Self {
        Self {
            id: local.id,
            name: local.name,
            color: local.color,
            is_favorite: local.is_favorite,
        }
    }
}

impl From<LocalTask> for TaskDisplay {
    fn from(local: LocalTask) -> Self {
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
            labels: serde_json::from_str(&local.labels).unwrap_or_default(),
            description: local.description,
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
            is_recurring: task.due.as_ref().is_some_and(|d| d.is_recurring),
            deadline: task.deadline.map(|d| d.date),
            duration: duration_string,
            labels: serde_json::to_string(&task.labels).unwrap_or_default(),
            description: task.description,
            last_synced: Utc::now(),
        }
    }
}

/// Local storage manager for Todoist data
pub struct LocalStorage {
    pool: SqlitePool,
}

impl LocalStorage {
    /// Initialize the local storage with `SQLite` database
    pub async fn new() -> Result<Self> {
        // Create data directory if it doesn't exist
        let data_dir = dirs::data_dir()
            .unwrap_or_else(|| std::env::current_dir().unwrap())
            .join("terminalist");

        // Ensure directory exists and is writable
        if let Err(e) = std::fs::create_dir_all(&data_dir) {
            eprintln!("Warning: Could not create data directory {}: {}", data_dir.display(), e);
            eprintln!("Falling back to current directory for database storage.");

            // Fallback to current directory
            let fallback_path = std::env::current_dir()?.join("terminalist.db");
            let database_url = format!("sqlite://{}?mode=rwc", fallback_path.display());

            let pool = SqlitePool::connect(&database_url).await?;
            let storage = Self { pool };
            storage.init_schema().await?;
            return Ok(storage);
        }

        let db_path = data_dir.join("terminalist.db");
        let database_url = format!("sqlite://{}?mode=rwc", db_path.display());

        // Try to connect to the database
        match SqlitePool::connect(&database_url).await {
            Ok(pool) => {
                let storage = Self { pool };
                storage.init_schema().await?;
                Ok(storage)
            }
            Err(e) => {
                eprintln!("Warning: Could not create database in data directory: {e}");
                eprintln!("Falling back to current directory for database storage.");

                // Fallback to current directory
                let fallback_path = std::env::current_dir()?.join("terminalist.db");
                let database_url = format!("sqlite://{}?mode=rwc", fallback_path.display());

                let pool = SqlitePool::connect(&database_url).await?;
                let storage = Self { pool };
                storage.init_schema().await?;
                Ok(storage)
            }
        }
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
                is_favorite BOOLEAN NOT NULL,
                is_inbox_project BOOLEAN NOT NULL,
                order_index INTEGER NOT NULL,
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
                project_id TEXT NOT NULL,
                is_completed BOOLEAN NOT NULL,
                is_deleted BOOLEAN NOT NULL,
                priority INTEGER NOT NULL,
                order_index INTEGER NOT NULL,
                due_date TEXT,
                due_datetime TEXT,
                is_recurring BOOLEAN NOT NULL,
                deadline TEXT,
                duration TEXT,
                labels TEXT NOT NULL,
                description TEXT,
                last_synced TEXT NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects(id)
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
        for project in projects {
            let local_project: LocalProject = project.into();
            sqlx::query(
                r"
                INSERT INTO projects (id, name, color, is_favorite, is_inbox_project, order_index, last_synced)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ",
            )
            .bind(&local_project.id)
            .bind(&local_project.name)
            .bind(&local_project.color)
            .bind(local_project.is_favorite)
            .bind(local_project.is_inbox_project)
            .bind(local_project.order_index)
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
                INSERT OR REPLACE INTO tasks (id, content, project_id, is_completed, is_deleted, priority, order_index, due_date, due_datetime, is_recurring, deadline, duration, labels, description, last_synced)
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

    /// Get all projects from local storage
    pub async fn get_projects(&self) -> Result<Vec<ProjectDisplay>> {
        let rows = sqlx::query("SELECT id, name, color, is_favorite FROM projects ORDER BY order_index, name")
            .fetch_all(&self.pool)
            .await?;

        let projects = rows
            .into_iter()
            .map(|row| ProjectDisplay {
                id: row.get("id"),
                name: row.get("name"),
                color: row.get("color"),
                is_favorite: row.get("is_favorite"),
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

        let tasks = rows
            .into_iter()
            .map(|row| TaskDisplay {
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
                labels: serde_json::from_str(&row.get::<String, _>("labels")).unwrap_or_default(),
                description: row.get("description"),
            })
            .collect();

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

        let tasks = rows
            .into_iter()
            .map(|row| TaskDisplay {
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
                labels: serde_json::from_str(&row.get::<String, _>("labels")).unwrap_or_default(),
                description: row.get("description"),
            })
            .collect();

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
}
