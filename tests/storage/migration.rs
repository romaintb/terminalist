//! Upgrade tests: opening a database that a *pre*-persistent-cache release left behind.
//!
//! Before this branch, every launch deleted `terminalist.db` and rebuilt it, so the file on
//! disk when the process exited always had the old shape: a `backends` row whose `uuid` is a
//! random v4 (there was no derived UUID and no `idx_backends_type_name`), with every cached
//! project/task keyed to that random UUID. `--debug` kept the file *and* inserted a fresh v4
//! row on every launch, so those installations accumulated duplicate `("todoist", "My Todoist")`
//! rows.
//!
//! These tests reproduce both shapes with raw SQL — deliberately *not* through
//! `LocalStorage::new_at`, which would produce the new shape — and then start the app against
//! them the way `main.rs` does. Every test works inside a `tempfile` directory; none of them
//! ever touches a real user path.

use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use terminalist::backend_registry::{derive_backend_uuid, BackendRegistry};
use terminalist::repositories::{BackendRepository, ProjectRepository, TaskRepository};
use terminalist::storage::{LocalStorage, DB_FILE_NAME};

/// SQLite stores `Uuid` columns as 16-byte blobs, so raw-SQL fixtures must bind blob literals.
fn blob(uuid: &Uuid) -> String {
    format!("X'{}'", uuid.simple())
}

async fn exec(conn: &DatabaseConnection, sql: &str) {
    conn.execute(Statement::from_string(DbBackend::Sqlite, sql.to_owned()))
        .await
        .unwrap_or_else(|e| panic!("fixture statement failed: {e}\n{sql}"));
}

/// Create the exact table set the pre-branch release created — same DDL, but **without**
/// `idx_backends_type_name`, which did not exist yet.
async fn create_legacy_schema(conn: &DatabaseConnection) {
    for sql in [
        r#"CREATE TABLE "backends" ( "uuid" uuid_text NOT NULL PRIMARY KEY, "backend_type" varchar NOT NULL, "name" varchar NOT NULL, "is_enabled" boolean NOT NULL, "credentials" varchar NOT NULL, "settings" varchar NOT NULL )"#,
        r#"CREATE TABLE "projects" ( "uuid" uuid_text NOT NULL PRIMARY KEY, "backend_uuid" uuid_text NOT NULL, "remote_id" varchar NOT NULL, "name" varchar NOT NULL, "is_favorite" boolean NOT NULL, "is_inbox_project" boolean NOT NULL, "order_index" integer NOT NULL, "parent_uuid" uuid_text, FOREIGN KEY ("parent_uuid") REFERENCES "projects" ("uuid"), FOREIGN KEY ("backend_uuid") REFERENCES "backends" ("uuid") ON DELETE CASCADE )"#,
        r#"CREATE TABLE "sections" ( "uuid" uuid_text NOT NULL PRIMARY KEY, "backend_uuid" uuid_text NOT NULL, "remote_id" varchar NOT NULL, "name" varchar NOT NULL, "project_uuid" uuid_text NOT NULL, "order_index" integer NOT NULL, FOREIGN KEY ("project_uuid") REFERENCES "projects" ("uuid") ON DELETE CASCADE, FOREIGN KEY ("backend_uuid") REFERENCES "backends" ("uuid") ON DELETE CASCADE )"#,
        r#"CREATE TABLE "labels" ( "uuid" uuid_text NOT NULL PRIMARY KEY, "backend_uuid" uuid_text NOT NULL, "remote_id" varchar NOT NULL, "name" varchar NOT NULL, "order_index" integer NOT NULL, "is_favorite" boolean NOT NULL, FOREIGN KEY ("backend_uuid") REFERENCES "backends" ("uuid") ON DELETE CASCADE )"#,
        r#"CREATE TABLE "tasks" ( "uuid" uuid_text NOT NULL PRIMARY KEY, "backend_uuid" uuid_text NOT NULL, "remote_id" varchar NOT NULL, "content" varchar NOT NULL, "description" varchar, "project_uuid" uuid_text NOT NULL, "section_uuid" uuid_text, "parent_uuid" uuid_text, "priority" integer NOT NULL, "order_index" integer NOT NULL, "due_date" varchar, "due_datetime" varchar, "is_recurring" boolean NOT NULL, "deadline" varchar, "duration" varchar, "is_completed" boolean NOT NULL, "is_deleted" boolean NOT NULL, FOREIGN KEY ("project_uuid") REFERENCES "projects" ("uuid") ON DELETE CASCADE, FOREIGN KEY ("section_uuid") REFERENCES "sections" ("uuid") ON DELETE SET NULL, FOREIGN KEY ("parent_uuid") REFERENCES "tasks" ("uuid") ON DELETE CASCADE, FOREIGN KEY ("backend_uuid") REFERENCES "backends" ("uuid") ON DELETE CASCADE )"#,
        r#"CREATE TABLE "task_labels" ( "task_uuid" uuid_text NOT NULL, "label_uuid" uuid_text NOT NULL, CONSTRAINT "pk-task_labels" PRIMARY KEY ("task_uuid", "label_uuid"), FOREIGN KEY ("task_uuid") REFERENCES "tasks" ("uuid") ON DELETE CASCADE, FOREIGN KEY ("label_uuid") REFERENCES "labels" ("uuid") ON DELETE CASCADE )"#,
        "CREATE UNIQUE INDEX idx_projects_backend_remote ON projects(backend_uuid, remote_id)",
        "CREATE UNIQUE INDEX idx_sections_backend_remote ON sections(backend_uuid, remote_id)",
        "CREATE UNIQUE INDEX idx_labels_backend_remote ON labels(backend_uuid, remote_id)",
        "CREATE UNIQUE INDEX idx_tasks_backend_remote ON tasks(backend_uuid, remote_id)",
    ] {
        exec(conn, sql).await;
    }
}

async fn insert_legacy_backend(conn: &DatabaseConnection, uuid: &Uuid) {
    exec(
        conn,
        &format!(
            r#"INSERT INTO backends (uuid, backend_type, name, is_enabled, credentials, settings)
               VALUES ({}, 'todoist', 'My Todoist', 1, '{{"api_token":"legacy-token"}}', '{{}}')"#,
            blob(uuid)
        ),
    )
    .await;
}

/// Seed one project plus one task under `backend_uuid`, named after `suffix` so tests can tell
/// which backend row's cache survived.
async fn insert_legacy_project_and_task(conn: &DatabaseConnection, backend_uuid: &Uuid, suffix: &str) -> (Uuid, Uuid) {
    let project_uuid = Uuid::new_v4();
    let task_uuid = Uuid::new_v4();

    exec(
        conn,
        &format!(
            r#"INSERT INTO projects (uuid, backend_uuid, remote_id, name, is_favorite, is_inbox_project, order_index, parent_uuid)
               VALUES ({}, {}, 'p-{suffix}', 'Project {suffix}', 0, 0, 0, NULL)"#,
            blob(&project_uuid),
            blob(backend_uuid)
        ),
    )
    .await;

    exec(
        conn,
        &format!(
            r#"INSERT INTO tasks (uuid, backend_uuid, remote_id, content, description, project_uuid, section_uuid,
                                  parent_uuid, priority, order_index, due_date, due_datetime, is_recurring, deadline,
                                  duration, is_completed, is_deleted)
               VALUES ({}, {}, 't-{suffix}', 'Task {suffix}', NULL, {}, NULL, NULL, 1, 0, NULL, NULL, 0, NULL, NULL, 0, 0)"#,
            blob(&task_uuid),
            blob(backend_uuid),
            blob(&project_uuid)
        ),
    )
    .await;

    (project_uuid, task_uuid)
}

async fn legacy_connection(dir: &Path) -> DatabaseConnection {
    let db_path = dir.join(DB_FILE_NAME);
    let conn = Database::connect(format!("sqlite:{}?mode=rwc", db_path.display()))
        .await
        .expect("open fixture database");
    exec(&conn, "PRAGMA foreign_keys = ON;").await;
    conn
}

/// Do exactly what `main.rs` does on startup, against `dir`.
async fn start_app(dir: &Path) -> anyhow::Result<(Arc<Mutex<LocalStorage>>, Uuid)> {
    let storage = Arc::new(Mutex::new(LocalStorage::new_at(dir).await?));
    let registry = BackendRegistry::new(storage.clone());
    registry.load_backends().await?;
    let uuid = registry
        .add_backend(
            "todoist".to_string(),
            "My Todoist".to_string(),
            r#"{"api_token":"fresh-token"}"#.to_string(),
            "{}".to_string(),
        )
        .await?;
    Ok((storage, uuid))
}

#[tokio::test]
async fn upgrading_a_pre_persistent_cache_database_adopts_the_legacy_backend_row() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let legacy_uuid = Uuid::new_v4();

    let fixture = legacy_connection(tmp.path()).await;
    create_legacy_schema(&fixture).await;
    insert_legacy_backend(&fixture, &legacy_uuid).await;
    insert_legacy_project_and_task(&fixture, &legacy_uuid, "one").await;
    insert_legacy_project_and_task(&fixture, &legacy_uuid, "two").await;
    fixture.close().await.expect("close fixture connection");

    // Startup must not fail: the derived v5 UUID does not collide with the legacy v4 one, so an
    // insert keyed to the derived UUID would violate `idx_backends_type_name` and abort `main`.
    let (storage, backend_uuid) = start_app(tmp.path()).await.expect("startup must succeed on upgrade");

    assert_eq!(
        backend_uuid,
        legacy_uuid,
        "the existing row's identity must be adopted, not replaced by the derived UUID {}",
        derive_backend_uuid("todoist", "My Todoist")
    );

    let guard = storage.lock().await;
    let backends = BackendRepository::get_all(&guard.conn).await.expect("query backends");
    assert_eq!(backends.len(), 1, "expected exactly one backend row, got {backends:?}");
    assert_eq!(
        backends[0].uuid, legacy_uuid,
        "the legacy row must be the surviving one"
    );

    // The cache hangs off `backend_uuid`; adopting keeps it attached instead of orphaning it
    // under a dead UUID (which would render as a fully duplicated task list).
    let mut projects = ProjectRepository::get_all(&guard.conn)
        .await
        .expect("query projects")
        .into_iter()
        .map(|p| p.remote_id)
        .collect::<Vec<_>>();
    projects.sort();
    assert_eq!(projects, vec!["p-one".to_string(), "p-two".to_string()]);

    let mut tasks = TaskRepository::get_all(&guard.conn)
        .await
        .expect("query tasks")
        .into_iter()
        .map(|t| t.remote_id)
        .collect::<Vec<_>>();
    tasks.sort();
    assert_eq!(tasks, vec!["t-one".to_string(), "t-two".to_string()]);
}

#[tokio::test]
async fn upgrading_a_debug_mode_database_with_duplicate_backend_rows_starts_and_keeps_the_oldest() {
    let tmp = tempfile::tempdir().expect("tempdir");
    // `--debug` kept the file and inserted a fresh v4 row per launch, so these accumulated.
    let oldest = Uuid::new_v4();
    let middle = Uuid::new_v4();
    let newest = Uuid::new_v4();

    let fixture = legacy_connection(tmp.path()).await;
    create_legacy_schema(&fixture).await;
    for uuid in [&oldest, &middle, &newest] {
        insert_legacy_backend(&fixture, uuid).await;
    }
    insert_legacy_project_and_task(&fixture, &oldest, "oldest").await;
    insert_legacy_project_and_task(&fixture, &middle, "middle").await;
    insert_legacy_project_and_task(&fixture, &newest, "newest").await;
    fixture.close().await.expect("close fixture connection");

    // `CREATE UNIQUE INDEX idx_backends_type_name` fails outright against duplicate rows, which
    // would kill the app inside `LocalStorage::new_at` before anything else could run.
    let (storage, backend_uuid) = start_app(tmp.path())
        .await
        .expect("startup must survive duplicate legacy backend rows");

    assert_eq!(backend_uuid, oldest, "the oldest row is the one the cache is keyed to");

    let guard = storage.lock().await;
    let backends = BackendRepository::get_all(&guard.conn).await.expect("query backends");
    assert_eq!(
        backends.len(),
        1,
        "duplicates must be collapsed to a single row, got {backends:?}"
    );
    assert_eq!(backends[0].uuid, oldest);

    // The younger rows' caches cascade away with them, so nothing is left duplicated.
    let projects = ProjectRepository::get_all(&guard.conn)
        .await
        .expect("query projects")
        .into_iter()
        .map(|p| p.remote_id)
        .collect::<Vec<_>>();
    assert_eq!(projects, vec!["p-oldest".to_string()]);

    let tasks = TaskRepository::get_all(&guard.conn)
        .await
        .expect("query tasks")
        .into_iter()
        .map(|t| t.remote_id)
        .collect::<Vec<_>>();
    assert_eq!(tasks, vec!["t-oldest".to_string()]);
}

#[tokio::test]
async fn a_fresh_database_still_gets_the_derived_backend_uuid() {
    // The adoption path must not weaken the derived-UUID guarantee for new installations.
    let tmp = tempfile::tempdir().expect("tempdir");

    let (storage, first) = start_app(tmp.path()).await.expect("first launch");
    assert_eq!(first, derive_backend_uuid("todoist", "My Todoist"));
    drop(storage);

    let (storage, second) = start_app(tmp.path()).await.expect("second launch");
    assert_eq!(second, first, "relaunch must reuse the same identity");
    let guard = storage.lock().await;
    let backends = BackendRepository::get_all(&guard.conn).await.expect("query backends");
    assert_eq!(backends.len(), 1);
}
