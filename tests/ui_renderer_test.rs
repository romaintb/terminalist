use terminalist::ui::core::EventHandler;

#[tokio::test]
async fn test_event_handling() {
    // Test that the event handler can be created
    let event_handler = EventHandler::new();

    // Initially should not need to render (just created)
    assert!(!event_handler.should_render());

    // After waiting, should be ready to render
    tokio::time::sleep(tokio::time::Duration::from_millis(17)).await;
    assert!(event_handler.should_render());
}
