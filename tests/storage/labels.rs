use terminalist::storage::LocalStorage;

#[tokio::test]
async fn test_labels_storage_creation() {
    // Test that we can create local storage for labels
    let result = LocalStorage::new(false).await;
    assert!(result.is_ok(), "LocalStorage should be created successfully");
}
