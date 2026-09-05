//! Whole-app startup, driven through AppComponent's real action loop.

#[tokio::test]
async fn startup_loads_cached_data_when_the_backend_is_unavailable() {
    use sea_orm::{EntityTrait, Set};
    use std::sync::Arc;
    use terminalist::config::Config;
    use terminalist::entities::{backend, project};
    use terminalist::storage::LocalStorage;
    use terminalist::sync::SyncService;
    use terminalist::ui::app_component::AppComponent;
    use tokio::sync::Mutex;
    use uuid::Uuid;

    let db_path = std::env::temp_dir().join(format!("terminalist-offline-{}.db", Uuid::new_v4()));
    let storage = LocalStorage::new_at(db_path.clone()).await.unwrap();
    let backend_uuid = Uuid::new_v4();

    backend::Entity::insert(backend::ActiveModel {
        uuid: Set(backend_uuid),
        backend_type: Set("test".to_string()),
        name: Set("Unavailable backend".to_string()),
        is_enabled: Set(true),
        credentials: Set("{}".to_string()),
        settings: Set("{}".to_string()),
    })
    .exec(&storage.conn)
    .await
    .unwrap();
    project::Entity::insert(project::ActiveModel {
        uuid: Set(Uuid::new_v4()),
        backend_uuid: Set(backend_uuid),
        remote_id: Set("cached-project".to_string()),
        name: Set("Cached project".to_string()),
        is_favorite: Set(false),
        is_inbox_project: Set(false),
        order_index: Set(1),
        parent_uuid: Set(None),
    })
    .exec(&storage.conn)
    .await
    .unwrap();

    let storage = Arc::new(Mutex::new(storage));
    let sync_service = SyncService::new_for_test(storage.clone(), backend_uuid);
    let mut app = AppComponent::new(sync_service, Config::default(), Vec::new());
    app.trigger_initial_sync();

    let startup_result = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let actions = app.process_background_actions();
            for action in actions {
                app.handle_app_action(action).await;
            }
            if app.total_projects() == 1 && app.has_toast() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;

    assert!(
        startup_result.is_ok(),
        "cached startup did not finish within five seconds"
    );
    assert_eq!(app.total_projects(), 1);
    assert_eq!(app.state.projects[0].name, "Cached project");
    assert!(
        app.has_toast(),
        "the failed sync should have left a notice in the corner"
    );

    drop(app); // TaskManager cancels its tasks on drop
    storage.lock().await.conn.clone().close().await.unwrap();
    std::fs::remove_file(db_path).unwrap();
}
