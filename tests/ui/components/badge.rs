use terminalist::ui::components::badge::*;

#[test]
fn test_create_paren_badge() {
    // Test that paren badge creation works
    let badge = create_paren_badge("test");
    assert!(
        badge.content.contains("(test)"),
        "Paren badge should contain parentheses"
    );
}
