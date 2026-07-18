use terminalist::ui::core::task_manager::TaskManager;

#[test]
fn test_task_manager_creation() {
    // Test that TaskManager can be created without panicking
    let _task_manager = TaskManager::new();
}

#[tokio::test]
async fn task_operations_expose_blocking_status_immediately() {
    let (mut manager, _receiver) = TaskManager::new();
    manager.spawn_task_operation(|| async { Ok("done".to_string()) }, "Completing 2 tasks".to_string());

    assert!(manager.has_blocking_work());
    assert_eq!(manager.processing_description().as_deref(), Some("Completing 2 tasks"));
    manager.cancel_all_tasks();
}
