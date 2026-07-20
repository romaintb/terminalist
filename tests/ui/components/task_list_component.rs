use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use terminalist::entities::{project, task};
use terminalist::ui::components::task_list_item_component::TaskListItemType;
use terminalist::ui::components::TaskListComponent;
use terminalist::ui::core::actions::{Action, SidebarSelection, TaskDueDate};
use terminalist::ui::core::Component;
use terminalist::utils::datetime;
use uuid::Uuid;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn project() -> project::Model {
    project::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: "project".to_string(),
        name: "Test".to_string(),
        is_favorite: false,
        is_inbox_project: false,
        order_index: 0,
        parent_uuid: None,
    }
}

fn task(content: &str, project_uuid: Uuid, is_completed: bool) -> task::Model {
    task::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: content.to_string(),
        content: content.to_string(),
        description: None,
        project_uuid,
        section_uuid: None,
        parent_uuid: None,
        priority: 1,
        order_index: 0,
        due_date: None,
        due_datetime: None,
        is_recurring: false,
        deadline: None,
        duration: None,
        is_completed,
        completed_at: None,
        is_deleted: false,
    }
}

fn component_with_tasks(tasks: Vec<task::Model>, project: project::Model) -> TaskListComponent {
    let mut component = TaskListComponent::new();
    let project_uuid = project.uuid;
    component.update_data(
        tasks,
        Vec::new(),
        vec![project],
        Vec::new(),
        SidebarSelection::Project(project_uuid),
    );
    component
}

#[test]
fn test_task_list_component_creation() {
    // Test that TaskListComponent can be created without panicking
    let _task_list = TaskListComponent::new();
}

#[test]
fn empty_state_wraps_within_the_task_pane() {
    let backend = TestBackend::new(32, 6);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut component = TaskListComponent::new();

    terminal.draw(|frame| component.render(frame, frame.area())).unwrap();

    let buffer = terminal.backend().buffer();
    let rendered = (0..buffer.area.height)
        .map(|y| (0..buffer.area.width).map(|x| buffer[(x, y)].symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("'r' to"));
    assert!(rendered.contains("sync."));
}

#[test]
fn today_shows_a_matching_subtask_when_its_parent_is_filtered_out() {
    let project = project();
    let parent = task("Parent task", project.uuid, false);
    let mut subtask = task("Due subtask", project.uuid, false);
    subtask.parent_uuid = Some(parent.uuid);
    subtask.due_date = Some(datetime::format_today());

    let mut component = TaskListComponent::new();
    component.update_all_tasks(vec![parent, subtask.clone()]);
    component.update_data(
        vec![subtask],
        Vec::new(),
        vec![project],
        Vec::new(),
        SidebarSelection::Today,
    );

    assert_eq!(component.visible_incomplete_task_count(), 1);
    let visible_subtask = component.items.iter().find_map(|item| match item {
        TaskListItemType::Task(item) => Some(item),
        _ => None,
    });
    assert_eq!(
        visible_subtask.and_then(|item| item.parent_context.as_deref()),
        Some("Parent task")
    );
}

#[test]
fn visible_count_excludes_completed_and_deleted_tasks() {
    let project = project();
    let pending = task("pending", project.uuid, false);
    let completed = task("completed", project.uuid, true);
    let mut deleted = task("deleted", project.uuid, false);
    deleted.is_deleted = true;
    let component = component_with_tasks(vec![pending, completed, deleted], project);

    assert_eq!(component.visible_incomplete_task_count(), 1);
}

#[test]
fn selected_completed_task_content_contrasts_with_highlight() {
    let project = project();
    let mut component = component_with_tasks(vec![task("completed task", project.uuid, true)], project);
    let backend = TestBackend::new(50, 5);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|frame| component.render(frame, frame.area())).unwrap();

    let buffer = terminal.backend().buffer();
    let row = (0..buffer.area.height)
        .find(|&y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .contains("completed task")
        })
        .expect("completed task should be rendered");
    let title_start = (0..buffer.area.width - "completed task".len() as u16)
        .find(|&x| {
            (x..x + "completed task".len() as u16)
                .map(|cell_x| buffer[(cell_x, row)].symbol())
                .collect::<String>()
                == "completed task"
        })
        .expect("completed task cells should be rendered");

    for x in title_start..title_start + "completed task".len() as u16 {
        let cell = &buffer[(x, row)];
        assert_eq!(cell.fg, ratatui::style::Color::White);
        assert_eq!(cell.bg, ratatui::style::Color::DarkGray);
        assert!(cell.modifier.contains(ratatui::style::Modifier::CROSSED_OUT));
    }
}

#[test]
fn marks_tasks_and_unschedules_all_marked_tasks() {
    let project = project();
    let first = task("first", project.uuid, false);
    let second = task("second", project.uuid, false);
    let mut component = component_with_tasks(vec![first.clone(), second.clone()], project);

    component.handle_key_events(key(KeyCode::Char('x')));
    component.handle_key_events(key(KeyCode::Char('j')));
    component.handle_key_events(key(KeyCode::Char('x')));
    assert_eq!(component.marked_task_count(), 2);

    let action = component.handle_key_events(key(KeyCode::Char('u')));
    match action {
        Action::SetTasksDueDate { task_ids, due_date } => {
            assert!(matches!(due_date, TaskDueDate::None));
            assert_eq!(task_ids.len(), 2);
            assert!(task_ids.contains(&first.uuid));
            assert!(task_ids.contains(&second.uuid));
        }
        other => panic!("unexpected action: {other:?}"),
    }
    assert_eq!(component.marked_task_count(), 0);
}

#[test]
fn completion_toggles_each_marked_task_according_to_its_state() {
    let project = project();
    let pending = task("pending", project.uuid, false);
    let completed = task("completed", project.uuid, true);
    let mut component = component_with_tasks(vec![pending.clone(), completed.clone()], project);

    component.handle_key_events(key(KeyCode::Char('x')));
    component.handle_key_events(key(KeyCode::Char('j')));
    component.handle_key_events(key(KeyCode::Char('x')));

    let action = component.handle_key_events(key(KeyCode::Char(' ')));
    match action {
        Action::ToggleTasks(tasks) => {
            assert!(tasks.contains(&(pending.uuid, false)));
            assert!(tasks.contains(&(completed.uuid, true)));
        }
        other => panic!("unexpected action: {other:?}"),
    }
}

#[test]
fn escape_clears_marks_without_quitting() {
    let project = project();
    let mut component = component_with_tasks(vec![task("first", project.uuid, false)], project);

    component.handle_key_events(key(KeyCode::Char('x')));
    let action = component.handle_key_events(key(KeyCode::Esc));

    assert!(matches!(action, Action::Consumed));
    assert_eq!(component.marked_task_count(), 0);
}
