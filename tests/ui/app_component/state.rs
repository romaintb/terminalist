//! AppState is a plain data holder, so there is little to check beyond the one
//! default that matters: starting up "loading" would paint a spinner forever.

use terminalist::ui::app_component::AppState;

#[test]
fn test_app_state_default() {
    // Test that AppState can be created with default values
    let state = AppState::default();
    assert!(!state.loading, "Default AppState should not be loading");
}
