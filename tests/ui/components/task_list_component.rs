use terminalist::entities::{project, section, task};
use terminalist::ui::components::task_list_item_component::TaskListItemType;
use terminalist::ui::components::TaskListComponent;
use terminalist::ui::core::SidebarSelection;
use uuid::Uuid;

#[test]
fn test_task_list_component_creation() {
    // Test that TaskListComponent can be created without panicking
    let _task_list = TaskListComponent::new();
}

fn project_model(uuid: Uuid) -> project::Model {
    project::Model {
        uuid,
        backend_uuid: Uuid::nil(),
        remote_id: "p1".to_string(),
        name: "Project".to_string(),
        is_favorite: false,
        is_inbox_project: false,
        order_index: 0,
        parent_uuid: None,
    }
}

fn section_model(uuid: Uuid, project_uuid: Uuid) -> section::Model {
    section::Model {
        uuid,
        backend_uuid: Uuid::nil(),
        remote_id: "s1".to_string(),
        name: "Section".to_string(),
        project_uuid,
        order_index: 0,
    }
}

fn task_model(project_uuid: Uuid, section_uuid: Option<Uuid>, content: &str) -> task::Model {
    task::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::nil(),
        remote_id: content.to_string(),
        content: content.to_string(),
        description: None,
        project_uuid,
        section_uuid,
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

fn rendered_task_contents(component: &TaskListComponent) -> Vec<String> {
    component
        .items
        .iter()
        .filter_map(|item| match item {
            TaskListItemType::Task(task_item) => Some(task_item.task.content.clone()),
            _ => None,
        })
        .collect()
}

/// A task pointing at a section we don't have loaded must still be rendered, not swallowed.
/// Happens on a torn read between the sections query and the tasks query across a sync commit,
/// or when a task's section belongs to another project.
#[test]
fn test_tasks_with_unknown_section_are_still_rendered() {
    let project_uuid = Uuid::new_v4();
    let known_section = Uuid::new_v4();
    let other_project_section = Uuid::new_v4();

    let mut component = TaskListComponent::new();
    component.update_data(
        vec![
            task_model(project_uuid, None, "loose"),
            task_model(project_uuid, Some(known_section), "in known section"),
            task_model(project_uuid, Some(Uuid::new_v4()), "section not loaded"),
            task_model(project_uuid, Some(other_project_section), "section of another project"),
        ],
        vec![
            section_model(known_section, project_uuid),
            section_model(other_project_section, Uuid::new_v4()),
        ],
        vec![project_model(project_uuid)],
        Vec::new(),
        SidebarSelection::Project(0),
    );

    let mut rendered = rendered_task_contents(&component);
    rendered.sort();
    assert_eq!(
        rendered,
        vec!["in known section", "loose", "section not loaded", "section of another project",]
    );
}
