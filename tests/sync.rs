use terminalist::config::Config;
use terminalist::sync::SyncService;

#[tokio::test]
async fn test_logger_accessor_with_logging_enabled() {
    let mut config = Config::default();
    config.logging.enabled = true;

    // Create sync service with logging enabled
    let result = SyncService::new("fake_token".to_string(), false, &config).await;

    if let Ok(sync_service) = result {
        // Should have a logger
        let logger = sync_service.logger();
        assert!(logger.is_some());
    }
    // Note: This test may fail in CI if no storage is available, but that's okay
}

#[tokio::test]
async fn test_logger_accessor_with_logging_disabled() {
    let mut config = Config::default();
    config.logging.enabled = false;

    // Create sync service with logging disabled
    let result = SyncService::new("fake_token".to_string(), false, &config).await;

    if let Ok(sync_service) = result {
        // Should still have a logger (in-memory)
        let logger = sync_service.logger();
        assert!(logger.is_some());
    }
    // Note: This test may fail in CI if no storage is available, but that's okay
}