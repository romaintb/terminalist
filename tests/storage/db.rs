use sea_orm::{ConnectionTrait, DbBackend, Statement};
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
        .execute(Statement::from_string(
            DbBackend::Sqlite,
            "INSERT INTO backends \
             (uuid, backend_type, name, is_enabled, credentials, settings) \
             VALUES ('00000000-0000-0000-0000-000000000001', 'test', 'Test', 1, '{}', '{}')"
                .to_owned(),
        ))
        .await
        .expect("first connection should remain writable after the second startup");

    first.conn.close().await.expect("first connection should close");
    second.conn.close().await.expect("second connection should close");
    std::fs::remove_file(db_path).expect("test database should be removed");
}

#[tokio::test]
async fn test_stale_schema_version_rebuilds_cache() {
    let db_path = std::env::temp_dir().join(format!("terminalist-schema-{}.db", uuid::Uuid::new_v4()));

    let storage = LocalStorage::new_at(db_path.clone())
        .await
        .expect("LocalStorage should be created successfully");
    storage
        .conn
        .execute(Statement::from_string(
            DbBackend::Sqlite,
            "INSERT INTO backends \
             (uuid, backend_type, name, is_enabled, credentials, settings) \
             VALUES ('00000000-0000-0000-0000-000000000002', 'test', 'Test', 1, '{}', '{}')"
                .to_owned(),
        ))
        .await
        .expect("cached row should be inserted");
    // A table from a revision that no longer has a matching entity.
    storage
        .conn
        .execute(Statement::from_string(
            DbBackend::Sqlite,
            "CREATE TABLE retired_entity (uuid TEXT PRIMARY KEY);".to_owned(),
        ))
        .await
        .expect("legacy table should be created");
    // Pretend the file was written by an older revision of the entities.
    storage
        .conn
        .execute(Statement::from_string(
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
        .query_one(Statement::from_string(
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
        .query_all(Statement::from_string(
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
    std::fs::remove_file(db_path).expect("test database should be removed");
}
