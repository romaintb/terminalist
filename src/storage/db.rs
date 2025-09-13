use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{
    sqlite::{SqliteConnection, SqlitePool, SqlitePoolOptions},
    Connection, Row,
};

use crate::todoist::TaskDisplay;

/// Local storage manager for Todoist data
pub struct LocalStorage {
    pub(crate) pool: SqlitePool,
    _anchor: SqliteConnection,
}

impl LocalStorage {
    /// Initialize the local storage with `SQLite` database
    pub async fn new() -> Result<Self> {
        let database_url = "sqlite:file:terminalist_memdb?mode=memory&cache=shared".to_string();

        let pool = SqlitePoolOptions::new()
            .min_connections(1)
            .max_connections(4)
            .idle_timeout(None) // avoid idle reaping
            .max_lifetime(None) // avoid lifetime rotation
            .connect(&database_url)
            .await?;

        // Anchor connection outside the pool
        let anchor = SqliteConnection::connect(&database_url).await?;

        let storage = LocalStorage { pool, _anchor: anchor };
        storage.init_schema().await?;
        storage.start_keepalive_task();

        Ok(storage)
    }

    /// Start a background task to keep the database connection alive
    fn start_keepalive_task(&self) {
        let pool = self.pool.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));

            loop {
                interval.tick().await;

                // Execute a simple query to keep the connection alive
                let _ = sqlx::query("SELECT 1").execute(&pool).await;
            }
        });
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

        // Create sections table
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS sections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                project_id TEXT NOT NULL,
                order_index INTEGER NOT NULL DEFAULT 0,
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
                section_id TEXT,
                parent_id TEXT,
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
                last_synced TEXT NOT NULL,
                FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE
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

    /// Create TaskDisplay from database row with label parsing
    pub(crate) fn task_display_from_row(row: &sqlx::sqlite::SqliteRow) -> TaskDisplay {
        // Parse labels from JSON string
        let label_names: Vec<String> = serde_json::from_str(&row.get::<String, _>("labels")).unwrap_or_default();

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
            section_id: row.get("section_id"),
            parent_id: row.get("parent_id"),
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
    }

    /// Check if the database has any data
    pub async fn has_data(&self) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
            .fetch_one(&self.pool)
            .await?;
        Ok(count > 0)
    }

    /// Get the last sync time for a specific data type
    pub async fn get_last_sync(&self, data_type: &str) -> Result<Option<DateTime<Utc>>> {
        let result: Option<String> = sqlx::query_scalar("SELECT value FROM sync_metadata WHERE key = ?")
            .bind(data_type)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(timestamp_str) = result {
            Ok(Some(timestamp_str.parse()?))
        } else {
            Ok(None)
        }
    }

    /// Clear all data from the database
    pub async fn clear_all_data(&self) -> Result<()> {
        sqlx::query("DELETE FROM tasks").execute(&self.pool).await?;
        sqlx::query("DELETE FROM projects")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM labels")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM sections")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM sync_metadata")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Update sync timestamp for a data type
    pub(crate) async fn update_sync_timestamp(&self, data_type: &str) -> Result<()> {
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
}
