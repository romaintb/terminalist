use anyhow::Result;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteConnection, SqlitePool, SqlitePoolOptions},
    Connection,
};
use std::str::FromStr;
use uuid::Uuid;

/// Local storage manager for Todoist data
pub struct LocalStorage {
    pub(crate) pool: SqlitePool,
    _anchor: SqliteConnection,
}

impl LocalStorage {
    /// Initialize the local storage with `SQLite` database
    pub async fn new(debug_mode: bool) -> Result<Self> {
        let database_url = if debug_mode {
            // File-backed database for debugging - ensure file exists
            let db_path = "terminalist_debug.db";
            if !std::path::Path::new(db_path).exists() {
                std::fs::File::create(db_path)?;
            }
            format!("sqlite:{}", db_path)
        } else {
            // In-memory database for normal operation
            "sqlite:file:terminalist_memdb?mode=memory&cache=shared".to_string()
        };

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
        // Create backend registry table first
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS backends (
                backend_id TEXT PRIMARY KEY,
                backend_type TEXT NOT NULL,
                name TEXT NOT NULL,
                enabled BOOLEAN NOT NULL DEFAULT 1,
                last_sync DATETIME,
                sync_error TEXT,
                config TEXT
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create projects table with backend tracking
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS projects (
                uuid TEXT PRIMARY KEY,
                backend_id TEXT NOT NULL DEFAULT 'todoist',
                external_id TEXT NOT NULL,
                name TEXT NOT NULL,
                color TEXT,
                is_favorite BOOLEAN NOT NULL DEFAULT 0,
                is_inbox_project BOOLEAN NOT NULL DEFAULT 0,
                order_index INTEGER NOT NULL DEFAULT 0,
                parent_uuid TEXT REFERENCES projects(uuid) ON DELETE CASCADE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (backend_id) REFERENCES backends(backend_id) ON DELETE CASCADE,
                UNIQUE(backend_id, external_id)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create sections table (backend is inferred from project)
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS sections (
                uuid TEXT PRIMARY KEY,
                external_id TEXT NOT NULL,
                name TEXT NOT NULL,
                project_uuid TEXT NOT NULL REFERENCES projects(uuid) ON DELETE CASCADE,
                order_index INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Note: Uniqueness for sections will be enforced at application level
        // since SQLite doesn't support subqueries in index expressions

        // Create tasks table (backend is inferred from project)
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS tasks (
                uuid TEXT PRIMARY KEY,
                external_id TEXT NOT NULL,
                content TEXT NOT NULL,
                description TEXT,
                project_uuid TEXT NOT NULL,
                section_uuid TEXT,
                parent_uuid TEXT,
                priority INTEGER NOT NULL DEFAULT 1,
                order_index INTEGER NOT NULL DEFAULT 0,
                due_date TEXT,
                due_datetime TEXT,
                is_recurring BOOLEAN NOT NULL DEFAULT 0,
                deadline TEXT,
                duration TEXT,
                is_completed BOOLEAN NOT NULL DEFAULT 0,
                is_deleted BOOLEAN NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (parent_uuid) REFERENCES tasks(uuid) ON DELETE CASCADE,
                FOREIGN KEY (project_uuid) REFERENCES projects(uuid) ON DELETE CASCADE,
                FOREIGN KEY (section_uuid) REFERENCES sections(uuid) ON DELETE SET NULL
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Note: Uniqueness for tasks will be enforced at application level
        // since SQLite doesn't support subqueries in index expressions

        // Create labels table with backend tracking
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS labels (
                uuid TEXT PRIMARY KEY,
                backend_id TEXT NOT NULL DEFAULT 'todoist',
                external_id TEXT NOT NULL,
                name TEXT NOT NULL,
                color TEXT NOT NULL,
                order_index INTEGER NOT NULL DEFAULT 0,
                is_favorite BOOLEAN NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (backend_id) REFERENCES backends(backend_id) ON DELETE CASCADE,
                UNIQUE(backend_id, external_id)
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create task_labels junction table for many-to-many relationship
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS task_labels (
                task_uuid TEXT NOT NULL,
                label_uuid TEXT NOT NULL,
                PRIMARY KEY (task_uuid, label_uuid),
                FOREIGN KEY (task_uuid) REFERENCES tasks(uuid) ON DELETE CASCADE,
                FOREIGN KEY (label_uuid) REFERENCES labels(uuid) ON DELETE CASCADE
            )
            ",
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_task_labels_task_uuid ON task_labels(task_uuid)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_task_labels_label_uuid ON task_labels(label_uuid)")
            .execute(&self.pool)
            .await?;

        // Create indexes for tasks table
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_project_uuid ON tasks(project_uuid)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_section_uuid ON tasks(section_uuid)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_parent_uuid ON tasks(parent_uuid)")
            .execute(&self.pool)
            .await?;

        // Create indexes for backend tracking
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_projects_backend_id ON projects(backend_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_labels_backend_id ON labels(backend_id)")
            .execute(&self.pool)
            .await?;

        // Create indexes for external IDs (for efficient lookups during sync)
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_projects_external_id ON projects(backend_id, external_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_labels_external_id ON labels(backend_id, external_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_external_id ON tasks(external_id)")
            .execute(&self.pool)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_sections_external_id ON sections(external_id)")
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


    /// Generate a unique internal UUID for database entities
    pub fn generate_uuid() -> String {
        Uuid::new_v4().to_string()
    }

}

