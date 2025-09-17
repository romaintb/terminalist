use terminalist::ui::app_component::AppState;

#[test]
fn test_app_state_default() {
    // Test that AppState can be created with default values
    let state = AppState::default();
    assert!(!state.loading, "Default AppState should not be loading");
    assert!(state.error_message.is_none(), "Default AppState should have no error message");
}