use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

use terminalist::backend::BackendProject;
use terminalist::backend_registry::BackendRegistry;
use terminalist::config::Config;
use terminalist::storage::LocalStorage;
use terminalist::sync::{SyncService, SyncStatus};
use terminalist::ui::app_component::{AppComponent, AppState};
use terminalist::ui::core::actions::{Action, SelectionPolicy};
use terminalist::ui::core::{Component, SidebarSelection};

#[test]
fn test_app_state_default() {
    // Test that AppState can be created with default values
    let state = AppState::default();
    assert!(!state.loading, "Default AppState should not be loading");
    assert!(
        state.error_message.is_none(),
        "Default AppState should have no error message"
    );
}

// --- Selection stability across a completed sync -----------------------------------------
//
// These drive a real `AppComponent` over a `LocalStorage` in a `tempfile` directory (never a
// user path) with a `SyncService` in debug mode, so `trigger_initial_sync` schedules the local
// data load without ever touching the network. Background actions are awaited on the component's
// own channel, so every step is deterministic: no polling, no sleeping, no wall-clock timing.

fn project(remote_id: &str, name: &str) -> BackendProject {
    BackendProject {
        remote_id: remote_id.to_string(),
        name: name.to_string(),
        is_favorite: false,
        is_inbox: false,
        order_index: 0,
        parent_remote_id: None,
    }
}

/// Build an `AppComponent` backed by a seeded cache in `dir`, opening on `default_project`.
async fn new_app(dir: &Path, default_project: &str) -> AppComponent {
    let storage = Arc::new(Mutex::new(LocalStorage::new_at(dir).await.expect("open local storage")));
    let registry = Arc::new(BackendRegistry::new(storage.clone()));
    let backend_uuid = registry
        .add_backend(
            "todoist".to_string(),
            "app-component-test".to_string(),
            r#"{"api_token":"fake-token"}"#.to_string(),
            "{}".to_string(),
        )
        .await
        .expect("register fake backend");

    // `debug_mode = true` keeps `trigger_initial_sync` off the network; the sync completion
    // these tests care about is injected as an action instead.
    let sync_service = SyncService::new(registry, backend_uuid, true)
        .await
        .expect("construct sync service");

    let guard = storage.lock().await;
    sync_service
        .store_projects_batch(&guard, &[project("p1", "Alpha"), project("p2", "Beta")])
        .await
        .expect("seed projects");
    drop(guard);

    let mut config = Config::default();
    config.ui.default_project = default_project.to_string();

    AppComponent::new(sync_service, config, Vec::new())
}

/// Feed one action through the same path the render loop uses.
async fn step(app: &mut AppComponent, action: Action) {
    let processed = app.update(action);
    let _ = app.handle_app_action(processed).await;
}

/// Await the next background action and feed it in, returning it for assertions.
async fn pump(app: &mut AppComponent) -> Action {
    let action = app.next_background_action().await.expect("a background action");
    step(app, action.clone()).await;
    action
}

/// Startup: load the cache, then navigate away from the configured default view.
async fn start_and_navigate_away(app: &mut AppComponent) {
    app.trigger_initial_sync();

    let initial = pump(app).await;
    assert!(
        matches!(initial, Action::InitialDataLoaded { .. }),
        "startup should load the cache first, got {initial:?}"
    );
    assert_eq!(
        app.sidebar_selection(),
        &SidebarSelection::Upcoming,
        "the initial load should honour default_project"
    );
    pump(app).await; // the follow-up fetch for the newly established selection

    // The user navigates while the sync is still running — the whole point of the branch.
    step(app, Action::NavigateToSidebar(SidebarSelection::Project(0))).await;
    pump(app).await;
    assert_eq!(app.sidebar_selection(), &SidebarSelection::Project(0));
}

#[tokio::test]
async fn a_completed_sync_does_not_reset_the_sidebar_selection() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut app = new_app(tmp.path(), "upcoming").await;

    start_and_navigate_away(&mut app).await;

    // The background sync finishes. It must refresh the view, not re-run startup.
    step(&mut app, Action::SyncCompleted(SyncStatus::Success)).await;

    let refresh = app.next_background_action().await.expect("post-sync refresh");
    match &refresh {
        Action::DataLoaded { selection_policy, .. } => {
            assert_eq!(
                *selection_policy,
                SelectionPolicy::FollowTask,
                "a sync-delivered reload must anchor the task-list cursor to the selected task, \
                 since the user may be navigating while it lands"
            );
        }
        other => panic!("a completed sync must refresh through the selection-preserving path, got {other:?}"),
    }
    step(&mut app, refresh).await;

    assert_eq!(
        app.sidebar_selection(),
        &SidebarSelection::Project(0),
        "a completed sync must not snap the selection back to default_project"
    );
}

#[tokio::test]
async fn refresh_data_after_a_task_operation_does_not_move_the_cursor() {
    // Regression coverage at the origin, not just the mechanism: `Action::RefreshData` (sent
    // after a task operation completes, e.g. pressing `t` to mark an overdue task due today)
    // must reload with `SelectionPolicy::KeepIndex`, not `FollowTask`. A component-level test
    // alone would still pass if `RefreshData` were wired to `FollowTask` by mistake.
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut app = new_app(tmp.path(), "upcoming").await;

    start_and_navigate_away(&mut app).await;

    // Simulate what `spawn_task_operation` sends once a task operation (e.g. SetTaskDueToday)
    // completes: a plain `Action::RefreshData`, not a sync.
    step(&mut app, Action::RefreshData).await;

    let reload = app.next_background_action().await.expect("reload after RefreshData");
    match reload {
        Action::DataLoaded { selection_policy, .. } => {
            assert_eq!(
                selection_policy,
                SelectionPolicy::KeepIndex,
                "a reload triggered by a task operation must leave the task-list cursor on its row, \
                 not drag it along with the task that moved"
            );
        }
        other => panic!("expected Action::DataLoaded after RefreshData, got {other:?}"),
    }
}

#[tokio::test]
async fn a_stray_initial_data_load_does_not_reset_the_sidebar_selection() {
    // Belt and braces for the auto-sync timer: whatever reaches the component later, the
    // initial selection is established exactly once.
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut app = new_app(tmp.path(), "upcoming").await;

    start_and_navigate_away(&mut app).await;

    step(
        &mut app,
        Action::InitialDataLoaded {
            projects: Vec::new(),
            labels: Vec::new(),
            sections: Vec::new(),
            tasks: Vec::new(),
        },
    )
    .await;

    assert_eq!(
        app.sidebar_selection(),
        &SidebarSelection::Project(0),
        "only the first initial load may set the selection"
    );
}

#[tokio::test]
async fn a_sync_failure_does_not_strand_the_pending_initial_selection() {
    // The sync and the local load are independent now. A sync that fails fast (a bad token, no
    // network) must not stop the load that follows it from applying `default_project`.
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut app = new_app(tmp.path(), "upcoming").await;

    app.trigger_initial_sync();
    step(&mut app, Action::SyncFailed("no network".to_string())).await;

    let initial = pump(&mut app).await;
    assert!(matches!(initial, Action::InitialDataLoaded { .. }), "got {initial:?}");

    assert_eq!(
        app.sidebar_selection(),
        &SidebarSelection::Upcoming,
        "a failed sync must not leave startup stuck on the fallback selection"
    );
}
