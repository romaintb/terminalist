use terminalist::ui::core::task_manager::TaskManager;

#[test]
fn test_task_manager_creation() {
    // Test that TaskManager can be created without panicking
    let _task_manager = TaskManager::new();
}

/// is_syncing used to ask whether a task's description contained "sync", so anything the
/// user could name tripped it: a label called "sync", a search for "sync". Only a real
/// sync counts now, and these operations must not be mistaken for one.
#[tokio::test]
async fn operations_are_never_mistaken_for_a_sync() {
    let (mut manager, _actions) = TaskManager::new();
    assert!(!manager.is_syncing(), "a fresh manager is not syncing");

    for label in ["sync", "resync my notes", "Background sync"] {
        manager.spawn_task_operation(move || async move { Ok(format!("Create label: {label}")) }, None);
    }

    assert!(
        !manager.is_syncing(),
        "operations named after syncing are still not a sync"
    );
}
