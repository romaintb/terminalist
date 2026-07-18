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
