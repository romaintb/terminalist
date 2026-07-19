use sea_orm::{ConnectionTrait, Database, DbBackend, Statement, TryGetable};
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
async fn existing_database_gains_completed_at_cache_column() {
    let db_path = std::env::temp_dir().join(format!("terminalist-migration-{}.db", uuid::Uuid::new_v4()));
    let database_url = format!("sqlite:{}?mode=rwc", db_path.display());
    let old_database = Database::connect(database_url).await.expect("old database should open");
    old_database
        .execute(Statement::from_string(
            DbBackend::Sqlite,
            "CREATE TABLE tasks (
                uuid TEXT PRIMARY KEY NOT NULL,
                backend_uuid TEXT NOT NULL,
                remote_id TEXT NOT NULL
            )"
            .to_owned(),
        ))
        .await
        .expect("old tasks table should be created");
    old_database.close().await.expect("old database should close");

    let storage = LocalStorage::new_at(db_path.clone())
        .await
        .expect("existing database should migrate");
    let columns = storage
        .conn
        .query_all(Statement::from_string(
            DbBackend::Sqlite,
            "PRAGMA table_info(tasks)".to_owned(),
        ))
        .await
        .expect("task columns should be readable");
    assert!(columns.iter().any(|row| {
        String::try_get(row, "", "name")
            .map(|name| name == "completed_at")
            .unwrap_or(false)
    }));

    storage.conn.close().await.expect("database should close");
    std::fs::remove_file(db_path).expect("test database should be removed");
}
