use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use terminalist::entities::project;
use terminalist::ui::components::SidebarComponent;
use terminalist::ui::core::{Action, Component, SidebarSelection};
use uuid::Uuid;

fn project_model(name: &str) -> project::Model {
    project::Model {
        uuid: Uuid::new_v4(),
        backend_uuid: Uuid::nil(),
        remote_id: name.to_string(),
        name: name.to_string(),
        is_favorite: false,
        is_inbox_project: false,
        order_index: 0,
        parent_uuid: None,
    }
}

/// The sidebar re-derives its highlighted row from the selection on every reload. A sync
/// returning the projects in another order used to slide the highlight onto a neighbour.
#[test]
fn selection_follows_its_project_across_a_reorder() {
    let alpha = project_model("Alpha");
    let beta = project_model("Beta");

    let mut sidebar = SidebarComponent::new();
    sidebar.update_data(vec![alpha.clone(), beta.clone()], Vec::new());
    sidebar.selection = SidebarSelection::Project(beta.uuid);

    // A sync hands back the same projects in a different order.
    sidebar.update_data(vec![beta.clone(), alpha.clone()], Vec::new());

    // Sorted display order is Today, Tomorrow, Upcoming, Alpha, Beta, so the item before the
    // cursor is Alpha only if the cursor is still on Beta.
    let previous = sidebar.handle_key_events(KeyEvent::new(KeyCode::Char('K'), KeyModifiers::SHIFT));
    assert!(
        matches!(previous, Action::NavigateToSidebar(SidebarSelection::Project(uuid)) if uuid == alpha.uuid),
        "expected the cursor to still be on Beta, got {previous:?}"
    );
}
