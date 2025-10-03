use terminalist::storage::LocalStorage;

#[tokio::test]
async fn test_local_storage_creation() {
    // Test that we can create local storage (use in-memory database for tests)
    let result = LocalStorage::new(false).await;
    assert!(result.is_ok(), "LocalStorage should be created successfully");
}
