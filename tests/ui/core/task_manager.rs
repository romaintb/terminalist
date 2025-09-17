use terminalist::ui::core::task_manager::TaskManager;

#[test]
fn test_task_manager_creation() {
    // Test that TaskManager can be created
    let _task_manager = TaskManager::new();
    // Just test that it was created without panicking
    assert!(true, "TaskManager should be creatable");
}