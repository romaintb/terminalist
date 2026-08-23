use anyhow::{Context, Result};
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, Schema, Statement};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::entities::{backend, label, project, section, task, task_label};

/// Filename of the SQLite cache inside the data directory.
pub const DB_FILE_NAME: &str = "terminalist.db";

/// Resolve the directory that holds the local SQLite cache.
///
/// `None` yields the platform data directory (`dirs::data_dir()/terminalist`), which is the
/// historical location. A configured path is used as given, except that a leading `~` is
/// expanded to the user's home directory — without that, `data_dir = "~/foo"` would silently
/// create a directory literally named `~` in the current working directory. Relative paths
/// resolve against the process working directory.
///
/// # Errors
///
/// Returns an error if the platform data directory (or, for a `~` path, the home directory)
/// cannot be determined.
pub fn resolve_data_dir(configured: Option<&Path>) -> Result<PathBuf> {
    match configured {
        None => Ok(dirs::data_dir()
            .context("Failed to determine the platform data directory")?
            .join("terminalist")),
        Some(path) => expand_tilde(path),
    }
}

/// Expand a leading `~` / `~/` (and `~\` on Windows) to the home directory.
fn expand_tilde(path: &Path) -> Result<PathBuf> {
    let Some(text) = path.to_str() else {
        return Ok(path.to_path_buf());
    };

    let remainder = if text == "~" {
        Some("")
    } else {
        text.strip_prefix("~/").or_else(|| text.strip_prefix(r"~\"))
    };

    match remainder {
        None => Ok(path.to_path_buf()),
        Some(remainder) => {
            let home = dirs::home_dir().context("Failed to determine the home directory")?;
            Ok(if remainder.is_empty() {
                home
            } else {
                home.join(remainder)
            })
        }
    }
}

/// Restrict the database file to owner-only access on Unix.
///
/// `backends.credentials` holds the raw `TODOIST_API_TOKEN` as plaintext JSON, so the cache is
/// a credential file. Best-effort: a filesystem that cannot represent Unix modes (a mounted
/// exFAT/SMB share, which `data_dir` now lets users point at) must not stop the app from
/// starting, so a failure is logged rather than propagated. No-op on Windows.
#[cfg(unix)]
fn restrict_permissions(db_path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    if let Err(e) = std::fs::set_permissions(db_path, std::fs::Permissions::from_mode(0o600)) {
        log::warn!(
            "Failed to restrict permissions on {} (it holds your API token in plaintext): {e}",
            db_path.display()
        );
    }
}

#[cfg(not(unix))]
fn restrict_permissions(_db_path: &Path) {}

/// Local storage manager for Todoist data
pub struct LocalStorage {
    pub conn: DatabaseConnection,
}

impl LocalStorage {
    /// Open local storage in the platform's default data directory.
    pub async fn new() -> Result<Self> {
        Self::new_at(resolve_data_dir(None)?).await
    }

    /// Open local storage with the SQLite database inside `data_dir`.
    ///
    /// The directory and the database are created if missing, and the schema is created
    /// idempotently, so an existing cache is reused as-is. Data is reconciled against the
    /// backend by [`crate::sync::SyncService`] rather than rebuilt, so the cache survives
    /// across launches.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created, or the connection or schema
    /// setup fails.
    pub async fn new_at(data_dir: impl AsRef<Path>) -> Result<Self> {
        let data_dir = data_dir.as_ref();
        std::fs::create_dir_all(data_dir)
            .with_context(|| format!("Failed to create data directory: {}", data_dir.display()))?;

        let db_path = data_dir.join(DB_FILE_NAME);
        let database_url = format!("sqlite:{}?mode=rwc", db_path.display());

        let mut opt = ConnectOptions::new(database_url);
        opt.max_connections(4)
            .min_connections(1)
            .connect_timeout(Duration::from_secs(8))
            .idle_timeout(Duration::from_secs(3600))
            .sqlx_logging(false);

        let conn = Database::connect(opt).await?;

        conn.execute(Statement::from_string(
            DbBackend::Sqlite,
            "PRAGMA foreign_keys = ON;".to_owned(),
        ))
        .await?;

        restrict_permissions(&db_path);

        let storage = LocalStorage { conn };
        storage
            .init_schema()
            .await
            .with_context(|| format!("Failed to set up the database schema in {}", db_path.display()))?;

        Ok(storage)
    }

    /// Initialize database schema
    async fn init_schema(&self) -> Result<()> {
        let backend = self.conn.get_database_backend();
        let schema = Schema::new(backend);

        // Create tables in the correct order (parent tables first).
        let table_statements = vec![
            schema.create_table_from_entity(backend::Entity).if_not_exists().to_owned(),
            schema.create_table_from_entity(project::Entity).if_not_exists().to_owned(),
            schema.create_table_from_entity(section::Entity).if_not_exists().to_owned(),
            schema.create_table_from_entity(label::Entity).if_not_exists().to_owned(),
            schema.create_table_from_entity(task::Entity).if_not_exists().to_owned(),
            schema.create_table_from_entity(task_label::Entity).if_not_exists().to_owned(),
        ];

        for statement in table_statements {
            self.conn.execute(backend.build(&statement)).await?;
        }

        // Releases predating `idx_backends_type_name` had no uniqueness guarantee on
        // `(backend_type, name)`, and the old `--debug` mode kept the database file while
        // inserting a fresh backend row on every launch — so those installations hold a stack of
        // duplicate `("todoist", "My Todoist")` rows. `CREATE UNIQUE INDEX` fails outright
        // against them, which would abort startup here before the app could do anything about
        // it. Collapse the duplicates first, keeping the lowest `rowid`: that is the oldest row,
        // the one `add_backend` will adopt and the one the longest-lived cache is keyed to.
        // `backends.uuid` is a blob primary key rather than an INTEGER one, so the table has a
        // real implicit `rowid` and insertion order is preserved.
        //
        // The younger rows' cached projects/tasks/labels/sections go with them via
        // `ON DELETE CASCADE` (sqlx applies `PRAGMA foreign_keys = ON` to every pooled
        // connection by default, and `new_at` sets it explicitly too), which is the
        // point: leaving them behind under a `backend_uuid` no longer in `backends` is exactly
        // the duplicated-task-list failure the derived-UUID design exists to prevent. Whatever
        // survives is reconciled against the remote by the next sync anyway.
        //
        // This is a no-op once the index exists, so it costs one scan of a tiny table per launch.
        self.conn
            .execute(Statement::from_string(
                DbBackend::Sqlite,
                "DELETE FROM backends WHERE rowid NOT IN (SELECT MIN(rowid) FROM backends GROUP BY backend_type, name)"
                    .to_owned(),
            ))
            .await
            .context("Failed to collapse duplicate backend rows left by an older version")?;

        // Create composite unique indexes for (backend_uuid, remote_id)
        let indexes = vec![
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_backend_remote ON projects(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_sections_backend_remote ON sections(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_labels_backend_remote ON labels(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_tasks_backend_remote ON tasks(backend_uuid, remote_id)",
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_backends_type_name ON backends(backend_type, name)",
        ];

        for index_sql in indexes {
            self.conn
                .execute(Statement::from_string(DbBackend::Sqlite, index_sql.to_owned()))
                .await?;
        }

        Ok(())
    }
}
