use terminalist::ui::core::context::AppContext;

#[test]
fn test_app_context_creation() {
    // Test that AppContext is accessible and has a valid size
    let context_size = std::mem::size_of::<AppContext>();
    // AppContext should have a non-zero size
    assert!(context_size > 0, "AppContext should have a non-zero size");
}
