use anyhow::{Context, Result};
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, Schema};
use std::path::PathBuf;
use std::time::Duration;

use crate::entities::{backend, label, project, section, task, task_label};

/// Local storage manager for Todoist data
pub struct LocalStorage {
    pub conn: DatabaseConnection,
}

impl LocalStorage {
    /// Get the database file path using XDG directories
    fn get_db_path() -> Result<PathBuf> {
        // Always use XDG data directory
        let data_dir = dirs::data_dir().context("Failed to get XDG data directory")?;
        let app_data_dir = data_dir.join("terminalist");

        // Create directory if it doesn't exist
        std::fs::create_dir_all(&app_data_dir).context("Failed to create application data directory")?;

        Ok(app_data_dir.join("terminalist.db"))
    }

    /// Initialize the local storage with SQLite database
    pub async fn new(debug_mode: bool) -> Result<Self> {
        let db_path = Self::get_db_path()?;

        // Normal mode: the DB is a throwaway cache, wiped and rebuilt by initial sync each boot.
        // Debug mode: keep an existing file so manually-loaded data (e.g. demo_data.sql) survives
        // and is shown without syncing.
        let existing = db_path.exists();
        if !debug_mode && existing {
            std::fs::remove_file(&db_path)?;
        }

        let database_url = format!("sqlite:{}?mode=rwc", db_path.display());

        let mut opt = ConnectOptions::new(database_url);
        opt.max_connections(4)
            .min_connections(1)
            .connect_timeout(Duration::from_secs(8))
            .idle_timeout(Duration::from_secs(3600))
            .sqlx_logging(false);

        let conn = Database::connect(opt).await?;

        // Enable foreign keys for SQLite
        conn.execute_unprepared("PRAGMA foreign_keys = ON;").await?;

        let storage = LocalStorage { conn };
        // A kept debug file already has its schema; only build it for a fresh file.
        if !(debug_mode && existing) {
            storage.init_schema().await?;
        }

        Ok(storage)
    }

    /// Initialize database schema
    async fn init_schema(&self) -> Result<()> {
        let backend = self.conn.get_database_backend();
        let schema = Schema::new(backend);

        // Create tables in the correct order (parent tables first).
        let table_statements = vec![
            schema.create_table_from_entity(backend::Entity),
            schema.create_table_from_entity(project::Entity),
            schema.create_table_from_entity(section::Entity),
            schema.create_table_from_entity(label::Entity),
            schema.create_table_from_entity(task::Entity),
            schema.create_table_from_entity(task_label::Entity),
        ];

        for statement in table_statements {
            self.conn.execute(&statement).await?;
        }

        // Create composite unique indexes for (backend_uuid, remote_id)
        let indexes = vec![
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_backend_remote ON projects(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_sections_backend_remote ON sections(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_labels_backend_remote ON labels(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_backend_remote ON tasks(backend_uuid, remote_id)",
        ];

        for index_sql in indexes {
            self.conn.execute_unprepared(index_sql).await?;
        }

        Ok(())
    }
}
