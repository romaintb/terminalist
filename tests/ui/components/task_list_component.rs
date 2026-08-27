use terminalist::entities::task;
use terminalist::ui::components::task_list_item_component::ListItem;
use terminalist::ui::components::TaskListComponent;
use terminalist::ui::core::actions::SelectionPolicy;
use terminalist::ui::core::SidebarSelection;
use terminalist::utils::datetime;
use uuid::Uuid;

#[test]
fn test_task_list_component_creation() {
    // Test that TaskListComponent can be created without panicking
    let _task_list = TaskListComponent::new();
}

/// Build a minimal, valid task::Model for tests. Every task gets its own random uuid
/// and project_uuid, and no due date/section/parent, so it always renders as a plain
/// root-level row regardless of sidebar view.
fn make_task(content: &str) -> task::Model {
    task::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::new_v4(),
        remote_id: String::new(),
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

/// Like `make_task`, but with a due date set, for driving `SidebarSelection::Today`'s
/// Overdue/Today sectioning.
fn make_task_due(content: &str, due_date: &str) -> task::Model {
    task::Model {
        due_date: Some(due_date.to_string()),
        ..make_task(content)
    }
}

// SidebarSelection::Project(0) with an empty projects vec falls back to the
// unsectioned "simple items" builder, so every task in `tasks` becomes a plain
// selectable row in list order without needing due dates or matching projects/sections.
fn load(component: &mut TaskListComponent, tasks: Vec<task::Model>, policy: SelectionPolicy) {
    component.update_data(
        tasks,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        SidebarSelection::Project(0),
        policy,
    );
}

// SidebarSelection::Today groups tasks under "Overdue"/"Today" Header rows (with a
// Separator between them when both are present) via build_today_items, so this is what
// actually exercises non-selectable rows in the item list.
fn load_today(component: &mut TaskListComponent, tasks: Vec<task::Model>, policy: SelectionPolicy) {
    component.update_data(
        tasks,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        SidebarSelection::Today,
        policy,
    );
}

#[test]
fn selection_follows_the_task_across_a_reload() {
    let mut component = TaskListComponent::new();

    let task_a = make_task("Task A");
    let task_b = make_task("Task B");
    let task_b_uuid = task_b.uuid;

    load(
        &mut component,
        vec![task_a.clone(), task_b.clone()],
        SelectionPolicy::FollowTask,
    );

    // Select task B (logical index 1: task A, task B).
    component.selected_index = 1;
    assert_eq!(component.get_selected_task().map(|t| t.uuid), Some(task_b_uuid));

    // Reload with a new task inserted above task B, using the policy a sync-delivered
    // reload uses: FollowTask.
    let task_new = make_task("Newly synced task");
    load(
        &mut component,
        vec![task_new, task_a, task_b],
        SelectionPolicy::FollowTask,
    );

    // Selection must still be task B, not whatever now sits at logical index 1.
    assert_eq!(
        component.get_selected_task().map(|t| t.uuid),
        Some(task_b_uuid),
        "selection should follow task B, not stay pinned to its old index"
    );
}

#[test]
fn keep_index_leaves_the_cursor_on_the_row_across_a_reload() {
    // Pins the regression: pressing `t` to mark an overdue task due "today" must not drag
    // the cursor along with it. That reload is user-initiated (a task operation, not a
    // sync), so it uses KeepIndex, and `selected_index` must stay put even though a
    // different task now occupies that row.
    let mut component = TaskListComponent::new();

    let task_a = make_task("Task A");
    let task_b = make_task("Task B");
    let task_b_uuid = task_b.uuid;

    load(
        &mut component,
        vec![task_a.clone(), task_b.clone()],
        SelectionPolicy::FollowTask,
    );

    // Select task B (logical index 1: task A, task B).
    component.selected_index = 1;
    assert_eq!(component.get_selected_task().map(|t| t.uuid), Some(task_b_uuid));

    // Reload with a new task inserted between task A and task B -- i.e. right above task B,
    // at the row task B used to occupy (logical index 1) -- using KeepIndex.
    let task_new = make_task("Task moved into this section");
    let task_new_uuid = task_new.uuid;
    load(
        &mut component,
        vec![task_a, task_new, task_b],
        SelectionPolicy::KeepIndex,
    );

    // selected_index must be untouched...
    assert_eq!(
        component.selected_index, 1,
        "KeepIndex must leave selected_index unchanged"
    );
    // ...so the selected task is now whichever task occupies row 1, not task B.
    assert_eq!(
        component.get_selected_task().map(|t| t.uuid),
        Some(task_new_uuid),
        "KeepIndex should leave the cursor on row 1, now occupied by the newly inserted task, not follow task B"
    );
}

#[test]
fn selection_survives_the_selected_task_disappearing() {
    let mut component = TaskListComponent::new();

    let task_a = make_task("Task A");
    let task_b = make_task("Task B");

    load(
        &mut component,
        vec![task_a.clone(), task_b],
        SelectionPolicy::FollowTask,
    );

    // Select task B (logical index 1).
    component.selected_index = 1;
    assert_eq!(
        component.get_selected_task().map(|t| t.content.clone()),
        Some("Task B".to_string())
    );

    // Reload without task B at all (e.g. completed/deleted elsewhere).
    load(&mut component, vec![task_a], SelectionPolicy::FollowTask);

    // No panic, and the selection must land on a valid, in-bounds row.
    assert_eq!(component.selected_index, 0);
    assert!(component.get_selected_task().is_some());
}

#[test]
fn empty_reload_resets_selection_without_panicking() {
    let mut component = TaskListComponent::new();

    let task_a = make_task("Task A");
    load(&mut component, vec![task_a], SelectionPolicy::FollowTask);
    component.selected_index = 0;
    assert!(component.get_selected_task().is_some());

    // Reload with no tasks at all.
    load(&mut component, Vec::new(), SelectionPolicy::FollowTask);

    assert_eq!(component.selected_index, 0);
    assert!(component.get_selected_task().is_none());
    assert!(component.items.is_empty());
}

#[test]
fn selection_follows_the_task_across_a_reload_with_section_headers() {
    let mut component = TaskListComponent::new();

    // Safely in the past regardless of when this test runs, so it always lands in the
    // Overdue bucket.
    let overdue_task = make_task_due("Overdue task", "2000-01-01");
    let overdue_task_uuid = overdue_task.uuid;
    let today_task = make_task_due("Today task", &datetime::format_today());

    load_today(
        &mut component,
        vec![overdue_task.clone(), today_task.clone()],
        SelectionPolicy::FollowTask,
    );

    // Confirm the fixture genuinely produced header/separator rows (Header("Overdue"),
    // Task, Separator, Header("Today"), Task) rather than collapsing into a flat list —
    // otherwise this test would be indistinguishable from the flat-list cases above and
    // wouldn't exercise the logical/physical index distinction at all.
    assert_eq!(
        component.items.len(),
        5,
        "expected 2 tasks plus 3 non-selectable rows (Overdue header, separator, Today header), got {:?}",
        component.items
    );
    let non_selectable_count = component.items.iter().filter(|item| !item.is_selectable()).count();
    assert_eq!(
        non_selectable_count, 3,
        "expected 3 non-selectable rows in the item list, got {:?}",
        component.items
    );

    // Select the overdue task: it sits right below the Overdue header, at physical index 1
    // but logical index 0 (only the 2 Task rows count logically). Deliberately NOT the last
    // selectable row in the list — see below for why that matters.
    component.selected_index = 0;
    assert_eq!(component.get_selected_task().map(|t| t.uuid), Some(overdue_task_uuid));

    // Reload with a new overdue task added ahead of it. It lands in the earlier (Overdue)
    // section, so the previously selected overdue task shifts from physical index 1 to
    // physical index 2 (logical index 0 to logical index 1), while the still-last today task
    // keeps sitting at the highest logical/physical index either way. Anchoring the
    // previously-selected task to anything but its own uuid (e.g. mixing up a physical
    // position with a logical one) would land selection on the wrong task here — and,
    // because it is not the last selectable row, `update_list_state`'s out-of-range clamp
    // cannot coincidentally paper over the mistake by forcing it back onto the right task.
    let new_overdue_task = make_task_due("New overdue task", "2000-01-02");
    load_today(
        &mut component,
        vec![new_overdue_task, overdue_task, today_task],
        SelectionPolicy::FollowTask,
    );

    assert_eq!(
        component.items.len(),
        6,
        "expected 3 tasks plus 3 non-selectable rows after the reload, got {:?}",
        component.items
    );
    assert_eq!(
        component.get_selected_task().map(|t| t.uuid),
        Some(overdue_task_uuid),
        "selection should follow the overdue task even though header rows shifted its physical position"
    );
}
