use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, ConnectionTrait, DbBackend, EntityTrait, Statement};
use terminalist::entities::{backend, project, task};
use terminalist::storage::LocalStorage;

#[tokio::test]
async fn test_local_storage_creation() {
    let db_path = std::env::temp_dir().join(format!("terminalist-storage-{}.db", uuid::Uuid::new_v4()));

    let first = LocalStorage::new_at(db_path.clone())
        .await
        .expect("first LocalStorage should be created successfully");
    let second = LocalStorage::new_at(db_path.clone())
        .await
        .expect("second LocalStorage should reuse the existing database");

    // A second startup must not replace the file underneath the first connection.
    first
        .conn
        .execute_raw(Statement::from_string(
            DbBackend::Sqlite,
            "INSERT INTO backends \
             (uuid, backend_type, name, credentials) \
             VALUES ('00000000-0000-0000-0000-000000000001', 'test', 'Test', '{}')"
                .to_owned(),
        ))
        .await
        .expect("first connection should remain writable after the second startup");

    first.conn.close().await.expect("first connection should close");
    second.conn.close().await.expect("second connection should close");
    // Best-effort: sqlx closes the sqlite handle on a worker thread that can outlive
    // pool.close(), and Windows refuses to unlink a file that still has one open.
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn test_stale_schema_version_rebuilds_cache() {
    let db_path = std::env::temp_dir().join(format!("terminalist-schema-{}.db", uuid::Uuid::new_v4()));

    let storage = LocalStorage::new_at(db_path.clone())
        .await
        .expect("LocalStorage should be created successfully");
    storage
        .conn
        .execute_raw(Statement::from_string(
            DbBackend::Sqlite,
            "INSERT INTO backends \
             (uuid, backend_type, name, credentials) \
             VALUES ('00000000-0000-0000-0000-000000000002', 'test', 'Test', '{}')"
                .to_owned(),
        ))
        .await
        .expect("cached row should be inserted");
    // A table from a revision that no longer has a matching entity.
    storage
        .conn
        .execute_raw(Statement::from_string(
            DbBackend::Sqlite,
            "CREATE TABLE retired_entity (uuid TEXT PRIMARY KEY);".to_owned(),
        ))
        .await
        .expect("legacy table should be created");
    // Pretend the file was written by an older revision of the entities.
    storage
        .conn
        .execute_raw(Statement::from_string(
            DbBackend::Sqlite,
            "PRAGMA user_version = 0;".to_owned(),
        ))
        .await
        .expect("user_version should be writable");
    storage.conn.close().await.expect("connection should close");

    let reopened = LocalStorage::new_at(db_path.clone())
        .await
        .expect("stale cache should be rebuilt, not rejected");
    let count = reopened
        .conn
        .query_one_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT COUNT(*) AS count FROM backends;".to_owned(),
        ))
        .await
        .expect("backends table should exist after the rebuild")
        .expect("count query should return a row")
        .try_get::<i64>("", "count")
        .expect("count should be readable");
    assert_eq!(count, 0, "a stale cache must be dropped, not kept");

    let leftovers = reopened
        .conn
        .query_all_raw(Statement::from_string(
            DbBackend::Sqlite,
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'retired_entity';".to_owned(),
        ))
        .await
        .expect("sqlite_master should be queryable");
    assert!(
        leftovers.is_empty(),
        "tables without a matching entity must be dropped too"
    );

    reopened.conn.close().await.expect("connection should close");
    // Best-effort: sqlx closes the sqlite handle on a worker thread that can outlive
    // pool.close(), and Windows refuses to unlink a file that still has one open.
    let _ = std::fs::remove_file(db_path);
}

#[cfg(unix)]
#[tokio::test]
async fn test_database_file_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;

    let db_path = std::env::temp_dir().join(format!("terminalist-perms-{}.db", uuid::Uuid::new_v4()));

    let storage = LocalStorage::new_at(db_path.clone())
        .await
        .expect("LocalStorage should be created successfully");
    storage.conn.close().await.expect("connection should close");

    let mode = std::fs::metadata(&db_path)
        .expect("database file should exist")
        .permissions()
        .mode();
    assert_eq!(
        mode & 0o777,
        0o600,
        "the cache holds the API token, it must not be readable by others"
    );

    let _ = std::fs::remove_file(db_path);
}

/// The sync layer upserts every remote record through `on_conflict`, so the composite
/// unique index `init_schema` builds has to be what resolves the conflict. Without it the
/// insert appends a duplicate row instead of updating the cached one.
#[tokio::test]
async fn test_upsert_updates_the_cached_row() {
    let db_path = std::env::temp_dir().join(format!("terminalist-upsert-{}.db", uuid::Uuid::new_v4()));

    let storage = LocalStorage::new_at(db_path.clone())
        .await
        .expect("LocalStorage should be created successfully");
    let conn = &storage.conn;

    let backend_uuid = uuid::Uuid::new_v4();
    backend::Entity::insert(backend::ActiveModel {
        uuid: ActiveValue::Set(backend_uuid),
        backend_type: ActiveValue::Set("test".into()),
        name: ActiveValue::Set("Test".into()),
        credentials: ActiveValue::Set("{}".into()),
    })
    .exec(conn)
    .await
    .expect("backend should be inserted");

    let project_uuid = uuid::Uuid::new_v4();
    project::Entity::insert(project::ActiveModel {
        uuid: ActiveValue::Set(project_uuid),
        backend_uuid: ActiveValue::Set(backend_uuid),
        remote_id: ActiveValue::Set("p1".into()),
        name: ActiveValue::Set("Project".into()),
        is_favorite: ActiveValue::Set(false),
        is_inbox_project: ActiveValue::Set(false),
        order_index: ActiveValue::Set(0),
        parent_uuid: ActiveValue::Set(None),
    })
    .exec(conn)
    .await
    .expect("project should be inserted");

    // Two syncs of the same remote task, each minting a fresh local uuid as the sync layer does.
    for content in ["first fetch", "second fetch"] {
        task::Entity::insert(task::ActiveModel {
            uuid: ActiveValue::Set(uuid::Uuid::new_v4()),
            backend_uuid: ActiveValue::Set(backend_uuid),
            remote_id: ActiveValue::Set("t1".into()),
            content: ActiveValue::Set(content.into()),
            description: ActiveValue::Set(None),
            project_uuid: ActiveValue::Set(project_uuid),
            section_uuid: ActiveValue::Set(None),
            parent_uuid: ActiveValue::Set(None),
            priority: ActiveValue::Set(1),
            order_index: ActiveValue::Set(0),
            due_date: ActiveValue::Set(None),
            due_datetime: ActiveValue::Set(None),
            is_recurring: ActiveValue::Set(false),
            deadline: ActiveValue::Set(None),
            duration: ActiveValue::Set(None),
            is_completed: ActiveValue::Set(false),
            is_deleted: ActiveValue::Set(false),
        })
        .on_conflict(
            OnConflict::columns([task::Column::BackendUuid, task::Column::RemoteId])
                .update_columns([task::Column::Content])
                .to_owned(),
        )
        .exec(conn)
        .await
        .expect("task should be upserted");
    }

    let tasks = task::Entity::find().all(conn).await.expect("tasks should be readable");
    assert_eq!(tasks.len(), 1, "a re-synced task must update, not duplicate");
    assert_eq!(tasks[0].content, "second fetch");

    storage.conn.close().await.expect("connection should close");
    // Best-effort: sqlx closes the sqlite handle on a worker thread that can outlive
    // pool.close(), and Windows refuses to unlink a file that still has one open.
    let _ = std::fs::remove_file(db_path);
}
