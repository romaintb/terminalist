use anyhow::Result;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteConnection, SqlitePool, SqlitePoolOptions},
    Connection,
};
use std::str::FromStr;

/// Local storage manager for Todoist data
pub struct LocalStorage {
    pub(crate) pool: SqlitePool,
    _anchor: SqliteConnection,
}

impl LocalStorage {
    /// Initialize the local storage with `SQLite` database
    pub async fn new() -> Result<Self> {
        let database_url = "sqlite:file:terminalist_memdb?mode=memory&cache=shared".to_string();

        // Configure SQLite connection options with foreign keys enabled
        let connect_options = SqliteConnectOptions::from_str(&database_url)?.foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .min_connections(1)
            .max_connections(4)
            .idle_timeout(None) // avoid idle reaping
            .max_lifetime(None) // avoid lifetime rotation
            .connect_with(connect_options.clone())
            .await?;

        // Anchor connection outside the pool
        let anchor = SqliteConnection::connect_with(&connect_options).await?;

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
                parent_id TEXT
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
                order_index INTEGER NOT NULL DEFAULT 0
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
                priority INTEGER NOT NULL DEFAULT 1,
                order_index INTEGER NOT NULL DEFAULT 0,
                due_date TEXT,
                due_datetime TEXT,
                is_recurring BOOLEAN NOT NULL DEFAULT 0,
                deadline TEXT,
                duration TEXT,
                FOREIGN KEY (parent_id) REFERENCES tasks(id) ON DELETE CASCADE,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                FOREIGN KEY (section_id) REFERENCES sections(id) ON DELETE SET NULL
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
                is_favorite BOOLEAN NOT NULL DEFAULT 0
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create task_labels junction table for many-to-many relationship
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS task_labels (
                task_id TEXT NOT NULL,
                label_id TEXT NOT NULL,
                PRIMARY KEY (task_id, label_id),
                FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
                FOREIGN KEY (label_id) REFERENCES labels(id) ON DELETE CASCADE
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_task_labels_task_id ON task_labels(task_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_task_labels_label_id ON task_labels(label_id)")
            .execute(&self.pool)
            .await?;

        // Create indexes for tasks table
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_project_id ON tasks(project_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_section_id ON tasks(section_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_parent_id ON tasks(parent_id)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Check if the database has any data
    pub async fn has_data(&self) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM projects")
            .fetch_one(&self.pool)
            .await?;
        Ok(count > 0)
    }

    /// Clear all data from the database
    pub async fn clear_all_data(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM task_labels").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM tasks").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM sections").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM projects").execute(&mut *tx).await?;
        sqlx::query("DELETE FROM labels").execute(&mut *tx).await?;
        tx.commit().await?;

        Ok(())
    }
}
