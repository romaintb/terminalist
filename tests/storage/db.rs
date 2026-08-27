use std::path::{Path, PathBuf};
use terminalist::storage::{resolve_data_dir, DB_FILE_NAME};

#[test]
fn test_resolve_data_dir_defaults_to_platform_directory() {
    let expected = dirs::data_dir().expect("platform data dir").join("terminalist");
    assert_eq!(resolve_data_dir(None).unwrap(), expected);
}

#[test]
fn test_resolve_data_dir_passes_through_absolute_paths() {
    let absolute = if cfg!(windows) {
        PathBuf::from(r"C:\tmp\terminalist-test")
    } else {
        PathBuf::from("/tmp/terminalist-test")
    };

    assert_eq!(resolve_data_dir(Some(&absolute)).unwrap(), absolute);
}

#[test]
fn test_resolve_data_dir_passes_through_relative_paths() {
    let relative = Path::new("terminalist-data");
    assert_eq!(resolve_data_dir(Some(relative)).unwrap(), relative);
}

#[test]
fn test_resolve_data_dir_expands_leading_tilde() {
    let Some(home) = dirs::home_dir() else {
        return; // No home directory in this environment; nothing to assert.
    };

    assert_eq!(resolve_data_dir(Some(Path::new("~"))).unwrap(), home);
    assert_eq!(
        resolve_data_dir(Some(Path::new("~/terminalist-dev"))).unwrap(),
        home.join("terminalist-dev")
    );
}

#[test]
fn test_resolve_data_dir_only_treats_leading_tilde_as_special() {
    // A tilde anywhere but the start is an ordinary path character.
    let embedded = Path::new("backups/~archive");
    assert_eq!(resolve_data_dir(Some(embedded)).unwrap(), embedded);
}

#[test]
fn test_db_file_name_is_stable() {
    // The filename is an implementation detail of the data directory, but tests and
    // docs both depend on this exact value.
    assert_eq!(DB_FILE_NAME, "terminalist.db");
}

#[test]
fn test_configured_data_dir_flows_from_config_to_resolution() {
    // Pins the config -> resolve_data_dir seam: the config layer, the resolver, and
    // `new_at` are each tested in isolation elsewhere, but nothing else joins them. If
    // main.rs stopped passing `config.storage.data_dir` through, this is what would catch it.
    let tmp = tempfile::tempdir().unwrap();
    let toml = tmp.path().join("terminalist.toml");
    let configured_dir = tmp.path().join("cache");
    std::fs::write(&toml, format!("[storage]\ndata_dir = {:?}\n", configured_dir)).unwrap();

    let (config, _) = terminalist::config::Config::load_from_file(&toml).unwrap();

    assert_eq!(
        resolve_data_dir(config.storage.data_dir.as_deref()).unwrap(),
        configured_dir
    );
}

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use terminalist::storage::LocalStorage;

/// Close the pool explicitly. Windows cannot delete a file with an open handle, so a
/// reopen-after-delete test is flaky unless the previous connection is closed first.
async fn close(storage: LocalStorage) {
    storage.conn.close().await.expect("close connection");
}

async fn create_marker(storage: &LocalStorage) {
    storage
        .conn
        .execute(Statement::from_string(
            DbBackend::Sqlite,
            "CREATE TABLE marker (id INTEGER)".to_owned(),
        ))
        .await
        .expect("create marker table");
}

async fn marker_exists(storage: &LocalStorage) -> bool {
    storage
        .conn
        .query_one(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type='table' AND name='marker'".to_owned(),
        ))
        .await
        .expect("query sqlite_master")
        .is_some()
}

#[tokio::test]
async fn test_new_at_opens_the_database_in_the_given_directory() {
    let tmp = tempfile::tempdir().unwrap();

    let storage = LocalStorage::new_at(tmp.path()).await.unwrap();
    close(storage).await;

    // The whole point of this feature: the configured directory is where the database
    // lives, so nothing ever touches the platform default path.
    assert!(tmp.path().join(DB_FILE_NAME).exists());
    assert_ne!(resolve_data_dir(None).unwrap(), tmp.path());
}

#[tokio::test]
async fn test_new_at_creates_missing_directories() {
    let tmp = tempfile::tempdir().unwrap();
    let nested = tmp.path().join("nested").join("data");

    let storage = LocalStorage::new_at(&nested).await.unwrap();
    close(storage).await;

    assert!(nested.join(DB_FILE_NAME).exists());
}

#[tokio::test]
async fn test_reopening_preserves_data() {
    let tmp = tempfile::tempdir().unwrap();

    let first = LocalStorage::new_at(tmp.path()).await.unwrap();
    create_marker(&first).await;
    close(first).await;

    // A normal open must no longer wipe the cache — this is the whole feature.
    let second = LocalStorage::new_at(tmp.path()).await.unwrap();
    assert!(marker_exists(&second).await, "reopening must preserve the cache");
    close(second).await;
}

/// The cache holds `backends.credentials` — the raw `TODOIST_API_TOKEN` — as plaintext JSON,
/// and `[storage] data_dir` now lets users relocate it, so the file must not be world-readable.
#[cfg(unix)]
#[tokio::test]
async fn test_database_file_is_readable_only_by_its_owner() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = tempfile::tempdir().unwrap();
    let storage = LocalStorage::new_at(tmp.path()).await.unwrap();
    close(storage).await;

    let mode = std::fs::metadata(tmp.path().join(DB_FILE_NAME)).unwrap().permissions().mode();

    assert_eq!(
        mode & 0o777,
        0o600,
        "the database stores the API token in plaintext and must be owner-only"
    );
}

#[tokio::test]
async fn test_schema_creation_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();

    // Second open re-runs init_schema against existing tables; without
    // if_not_exists this fails with "table projects already exists".
    let first = LocalStorage::new_at(tmp.path()).await.unwrap();
    close(first).await;
    let second = LocalStorage::new_at(tmp.path()).await.unwrap();
    close(second).await;
}

#[test]
fn test_backend_uuid_is_deterministic() {
    use terminalist::backend_registry::derive_backend_uuid;

    // The same (type, name) must yield the same UUID on every launch, or a
    // persistent cache duplicates every row under a fresh backend_uuid.
    assert_eq!(
        derive_backend_uuid("todoist", "My Todoist"),
        derive_backend_uuid("todoist", "My Todoist")
    );
    assert_ne!(
        derive_backend_uuid("todoist", "My Todoist"),
        derive_backend_uuid("todoist", "Work")
    );
    assert_ne!(
        derive_backend_uuid("todoist", "My Todoist"),
        derive_backend_uuid("ticktick", "My Todoist")
    );
}

#[tokio::test]
async fn test_relaunch_upsert_refreshes_credentials_without_re_enabling() {
    use sea_orm::EntityTrait;
    use std::sync::Arc;
    use terminalist::backend_registry::{derive_backend_uuid, BackendRegistry};
    use terminalist::entities::backend;
    use tokio::sync::Mutex;

    let tmp = tempfile::tempdir().unwrap();
    let storage = Arc::new(Mutex::new(LocalStorage::new_at(tmp.path()).await.unwrap()));
    let registry = BackendRegistry::new(storage.clone());

    let credentials_a = serde_json::json!({ "api_token": "token-a" }).to_string();
    let uuid = registry
        .add_backend(
            "todoist".to_string(),
            "My Todoist".to_string(),
            credentials_a,
            "{}".to_string(),
        )
        .await
        .unwrap();

    registry.disable_backend(&uuid).await.unwrap();

    // Simulate a relaunch with the same (type, name) but a rotated token.
    let credentials_b = serde_json::json!({ "api_token": "token-b" }).to_string();
    registry
        .add_backend(
            "todoist".to_string(),
            "My Todoist".to_string(),
            credentials_b.clone(),
            "{}".to_string(),
        )
        .await
        .unwrap();

    let locked = storage.lock().await;
    let rows = backend::Entity::find().all(&locked.conn).await.unwrap();

    assert_eq!(rows.len(), 1, "relaunch must upsert the existing row, not duplicate it");
    assert_eq!(rows[0].uuid, derive_backend_uuid("todoist", "My Todoist"));
    assert_eq!(
        rows[0].credentials, credentials_b,
        "relaunch must refresh the rotated token"
    );
    assert!(
        !rows[0].is_enabled,
        "relaunch must not silently re-enable a backend the user disabled"
    );
}
