use terminalist::ui::core::actions::Action;

#[test]
fn test_action_enum_exists() {
    // Test that Action enum is accessible and has a valid size
    let action_size = std::mem::size_of::<Action>();
    // Action enum should have a non-zero size
    assert!(action_size > 0, "Action enum should have a non-zero size");
}
