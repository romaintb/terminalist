//! AppState is a plain data holder, so there is little to check beyond the one
//! default that matters: starting up "loading" would paint a spinner forever.

use terminalist::ui::app_component::AppState;

#[test]
fn test_app_state_default() {
    // Test that AppState can be created with default values
    let state = AppState::default();
    assert!(!state.loading, "Default AppState should not be loading");
}

/// A project deleted from another client is simply absent from the next sync, which leaves
/// the sidebar selection naming something that no longer exists.
#[test]
fn selection_is_live_only_while_its_project_exists() {
    use terminalist::entities::project;
    use terminalist::ui::core::SidebarSelection;
    use uuid::Uuid;

    let uuid = Uuid::new_v4();
    let mut state = AppState::default();
    assert!(state.selection_is_live(), "Today always exists");

    state.sidebar_selection = SidebarSelection::Project(uuid);
    assert!(!state.selection_is_live());

    state.projects = vec![project::Model {
        uuid,
        backend_uuid: Uuid::nil(),
        remote_id: "p1".to_string(),
        name: "Project".to_string(),
        is_favorite: false,
        is_inbox_project: false,
        order_index: 0,
        parent_uuid: None,
    }];
    assert!(state.selection_is_live());
}
