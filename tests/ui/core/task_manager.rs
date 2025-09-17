use terminalist::ui::core::task_manager::TaskManager;

#[test]
fn test_task_manager_creation() {
    // Test that TaskManager can be created without panicking
    let _task_manager = TaskManager::new();
}
