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
        completed_at: None,
        is_deleted: false,
        deleted_at: None,
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

    dialog.handle_key_events(key(KeyCode::Down));
    assert!(dialog.search_results_focused);
    assert_eq!(dialog.search_selected_index, 0);
    dialog.handle_key_events(key(KeyCode::Down));
    assert_eq!(dialog.search_selected_index, 1);
    dialog.handle_key_events(key(KeyCode::Down));
    assert_eq!(dialog.search_selected_index, 1);

    dialog.handle_key_events(key(KeyCode::Up));
    assert_eq!(dialog.search_selected_index, 0);
    dialog.handle_key_events(key(KeyCode::Up));
    assert!(!dialog.search_results_focused);
    assert_eq!(dialog.search_selected_index, 0);
}

#[test]
fn test_all_letters_are_entered_while_search_input_is_focused() {
    let mut dialog = DialogComponent::new();
    dialog.dialog_type = Some(DialogType::TaskSearch);

    for character in 'a'..='z' {
        let action = dialog.handle_key_events(key(KeyCode::Char(character)));
        assert!(matches!(action, Action::SearchTasks(_)));
    }

    assert_eq!(dialog.input_buffer, "abcdefghijklmnopqrstuvwxyz");
    assert_eq!(dialog.cursor_position, 26);
}

#[test]
fn test_t_sets_selected_search_result_due_today() {
    let mut dialog = DialogComponent::new();
    dialog.dialog_type = Some(DialogType::TaskSearch);
    dialog.search_results = vec![search_task("first"), search_task("second")];
    dialog.search_selected_index = 1;
    dialog.search_results_focused = true;
    let selected_uuid = dialog.search_results[1].uuid;

    let action = dialog.handle_key_events(key(KeyCode::Char('t')));

    assert!(matches!(action, Action::SetTaskDueToday(uuid) if uuid == selected_uuid));
}

#[test]
fn test_space_completes_selected_search_result() {
    let mut dialog = DialogComponent::new();
    dialog.dialog_type = Some(DialogType::TaskSearch);
    dialog.search_results = vec![search_task("first"), search_task("second")];
    dialog.search_selected_index = 1;
    dialog.search_results_focused = true;
    let selected_uuid = dialog.search_results[1].uuid;

    let action = dialog.handle_key_events(key(KeyCode::Char(' ')));

    assert!(matches!(action, Action::CompleteTask(uuid) if uuid == selected_uuid));
}

#[test]
fn test_space_is_search_text_while_query_is_focused() {
    let mut dialog = DialogComponent::new();
    dialog.dialog_type = Some(DialogType::TaskSearch);

    let action = dialog.handle_key_events(key(KeyCode::Char(' ')));

    assert!(matches!(action, Action::SearchTasks(query) if query == " "));
    assert_eq!(dialog.input_buffer, " ");
}

#[test]
fn test_enter_has_no_search_action() {
    let mut dialog = DialogComponent::new();
    dialog.dialog_type = Some(DialogType::TaskSearch);

    assert!(matches!(dialog.handle_key_events(key(KeyCode::Enter)), Action::None));
}
