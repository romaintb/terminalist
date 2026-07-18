use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use terminalist::entities::task;
use terminalist::ui::components::DialogComponent;
use terminalist::ui::core::{Action, Component, DialogType};
use uuid::Uuid;

fn search_task(content: &str) -> task::Model {
    task::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: Uuid::new_v4().to_string(),
        content: content.to_string(),
        description: None,
        project_uuid: Uuid::new_v4(),
        section_uuid: None,
        parent_uuid: None,
        priority: 1,
        order_index: 0,
        due_date: None,
        due_datetime: None,
        is_recurring: false,
        deadline: None,
        duration: None,
        is_completed: false,
        is_deleted: false,
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn test_dialog_component_creation() {
    // Test that DialogComponent can be created without panicking
    let _dialog = DialogComponent::new();
}

#[test]
fn test_search_result_navigation_is_bounded() {
    let mut dialog = DialogComponent::new();
    dialog.dialog_type = Some(DialogType::TaskSearch);
    dialog.search_results = vec![search_task("first"), search_task("second")];

    dialog.handle_key_events(key(KeyCode::Char('j')));
    assert_eq!(dialog.search_selected_index, 1);
    dialog.handle_key_events(key(KeyCode::Down));
    assert_eq!(dialog.search_selected_index, 1);

    dialog.handle_key_events(key(KeyCode::Char('k')));
    assert_eq!(dialog.search_selected_index, 0);
    dialog.handle_key_events(key(KeyCode::Up));
    assert_eq!(dialog.search_selected_index, 0);
}

#[test]
fn test_t_sets_selected_search_result_due_today() {
    let mut dialog = DialogComponent::new();
    dialog.dialog_type = Some(DialogType::TaskSearch);
    dialog.search_results = vec![search_task("first"), search_task("second")];
    dialog.search_selected_index = 1;
    let selected_uuid = dialog.search_results[1].uuid;

    let action = dialog.handle_key_events(key(KeyCode::Char('t')));

    assert!(matches!(action, Action::SetTaskDueToday(uuid) if uuid == selected_uuid));
}
