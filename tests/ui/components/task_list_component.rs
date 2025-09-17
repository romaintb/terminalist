use terminalist::ui::components::TaskListComponent;

#[test]
fn test_task_list_component_creation() {
    // Test that TaskListComponent can be created
    let _task_list = TaskListComponent::new();
    // Just test that it was created without panicking
    assert!(true, "TaskListComponent should be creatable");
}