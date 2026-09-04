use anyhow::{Context, Result};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, Schema, Statement, TransactionTrait,
};
use std::path::PathBuf;
use std::time::Duration;

use crate::entities::{backend, label, project, section, task, task_label};

/// Schema revision of the local cache. Bump this whenever an entity definition changes.
///
/// The database is a disposable cache, not a source of truth, so a mismatch drops every
/// table and rebuilds from scratch instead of running a migration.
const SCHEMA_VERSION: i32 = 1;

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

    /// Initialize the local storage with the application SQLite database.
    ///
    /// The database is retained between runs and refreshed by the sync layer. Deleting it
    /// here would invalidate connections held by another running Terminalist process, so a
    /// schema change is handled by dropping the cached tables instead. See [`SCHEMA_VERSION`].
    pub async fn new(_debug_mode: bool) -> Result<Self> {
        let db_path = Self::get_db_path()?;
        Self::new_at(db_path).await
    }

    /// Open local storage at an explicit path.
    ///
    /// This is public so integration tests and alternate frontends can use an isolated
    /// database without touching the user's application data.
    pub async fn new_at(db_path: PathBuf) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        let database_url = format!("sqlite:{}?mode=rwc", db_path.display());

        let mut opt = ConnectOptions::new(database_url);
        opt.max_connections(4)
            .min_connections(1)
            .connect_timeout(Duration::from_secs(8))
            .idle_timeout(Duration::from_secs(3600))
            .sqlx_logging(false)
            .map_sqlx_sqlite_opts(|o| o.foreign_keys(true));

        let conn = Database::connect(opt).await?;

        let storage = LocalStorage { conn };
        storage.discard_stale_schema().await?;
        storage.init_schema().await?;

        Ok(storage)
    }

    /// Drop the cached tables when the file was written by a different schema revision.
    async fn discard_stale_schema(&self) -> Result<()> {
        let row = self
            .conn
            .query_one(Statement::from_string(
                DbBackend::Sqlite,
                "PRAGMA user_version;".to_owned(),
            ))
            .await?;
        let version = match row {
            Some(row) => row.try_get::<i32>("", "user_version")?,
            None => 0,
        };

        if version == SCHEMA_VERSION {
            return Ok(());
        }

        // The drops and the version bump commit together, so a crash can't leave a half-dropped
        // cache stamped with the new revision.
        let txn = self.conn.begin().await?;

        // Asking the file what it holds beats hardcoding a list: it also clears out tables from
        // revisions that no longer have a matching entity.
        //
        // Reverse creation order drops children before parents. Dropping a parent first leaves
        // the child holding a foreign key to a missing table, and its own drop then fails on
        // resolving that reference. sqlite_master lists objects in creation order, and
        // init_schema creates parents first, so walking it backwards is dependency order.
        let tables = txn
            .query_all(Statement::from_string(
                DbBackend::Sqlite,
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
                 ORDER BY rowid DESC;"
                    .to_owned(),
            ))
            .await?;

        for table in tables {
            let name = table.try_get::<String>("", "name")?;
            txn.execute(Statement::from_string(
                DbBackend::Sqlite,
                format!("DROP TABLE IF EXISTS \"{name}\";"),
            ))
            .await?;
        }

        // Not a bind parameter: SQLite only accepts a literal here, and the value is a constant.
        txn.execute(Statement::from_string(
            DbBackend::Sqlite,
            format!("PRAGMA user_version = {SCHEMA_VERSION};"),
        ))
        .await?;

        txn.commit().await?;

        Ok(())
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

        for mut statement in table_statements {
            statement.if_not_exists();
            self.conn.execute(backend.build(&statement)).await?;
        }

        // Create composite unique indexes for (backend_uuid, remote_id)
        let indexes = vec![
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_backend_remote ON projects(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_sections_backend_remote ON sections(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_labels_backend_remote ON labels(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_backend_remote ON tasks(backend_uuid, remote_id)",
        ];

        for index_sql in indexes {
            self.conn
                .execute(Statement::from_string(DbBackend::Sqlite, index_sql.to_owned()))
                .await?;
        }

        Ok(())
    }
}
