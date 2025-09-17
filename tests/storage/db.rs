use terminalist::storage::LocalStorage;

#[tokio::test]
async fn test_local_storage_creation() {
    // Test that we can create local storage
    let result = LocalStorage::new(true).await;
    assert!(result.is_ok(), "LocalStorage should be created successfully");
}
