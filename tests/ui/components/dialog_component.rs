use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use terminalist::entities::project;
use terminalist::ui::components::DialogComponent;
use terminalist::ui::core::{
    actions::{Action, DialogType},
    Component,
};
use uuid::Uuid;

fn project_model(uuid: Uuid, name: &str, is_inbox_project: bool) -> project::Model {
    project::Model {
        uuid,
        backend_uuid: Uuid::nil(),
        remote_id: name.to_string(),
        name: name.to_string(),
        is_favorite: false,
        is_inbox_project,
        order_index: 0,
        parent_uuid: None,
    }
}

fn press(dialog: &mut DialogComponent, code: KeyCode) -> Action {
    dialog.handle_key_events(KeyEvent::new(code, KeyModifiers::NONE))
}

/// Opens an edit dialog on a task living in `projects[0]`.
fn editing_dialog(projects: &[project::Model], task_uuid: Uuid) -> DialogComponent {
    let mut dialog = DialogComponent::new();
    dialog.update_data_with_tasks(projects.to_vec(), Vec::new(), Vec::new());
    dialog.update(Action::ShowDialog(DialogType::TaskEdit {
        task_uuid,
        content: "Buy milk".to_string(),
        project_uuid: projects[0].uuid,
    }));
    dialog
}

/// Tab must cycle the edit dialog through every project, inbox included, and wrap around.
#[test]
fn test_task_edit_tab_cycles_projects() {
    let inbox = project_model(Uuid::new_v4(), "Inbox", true);
    let work = project_model(Uuid::new_v4(), "Work", false);
    let home = project_model(Uuid::new_v4(), "Home", false);
    let projects = vec![inbox.clone(), work.clone(), home.clone()];

    let mut dialog = editing_dialog(&projects, Uuid::new_v4());
    assert_eq!(dialog.selected_task_project_uuid, Some(inbox.uuid));

    press(&mut dialog, KeyCode::Tab);
    assert_eq!(dialog.selected_task_project_uuid, Some(work.uuid));

    press(&mut dialog, KeyCode::Tab);
    assert_eq!(dialog.selected_task_project_uuid, Some(home.uuid));

    press(&mut dialog, KeyCode::Tab);
    assert_eq!(dialog.selected_task_project_uuid, Some(inbox.uuid));
}

/// Saving after Tab asks for the move; saving without Tab does not.
#[test]
fn test_task_edit_submits_move_only_when_project_changed() {
    let inbox = project_model(Uuid::new_v4(), "Inbox", true);
    let work = project_model(Uuid::new_v4(), "Work", false);
    let projects = vec![inbox.clone(), work.clone()];
    let task_uuid = Uuid::new_v4();

    let mut untouched = editing_dialog(&projects, task_uuid);
    match press(&mut untouched, KeyCode::Enter) {
        Action::EditTask { move_to_project, .. } => assert_eq!(move_to_project, None),
        other => panic!("expected EditTask, got {other:?}"),
    }

    let mut moved = editing_dialog(&projects, task_uuid);
    press(&mut moved, KeyCode::Tab);
    match press(&mut moved, KeyCode::Enter) {
        Action::EditTask { move_to_project, .. } => assert_eq!(move_to_project, Some(work.uuid)),
        other => panic!("expected EditTask, got {other:?}"),
    }
}
