use terminalist::utils::color::convert_todoist_color;

#[test]
fn test_convert_todoist_color() {
    // Test that color conversion function works
    let red_color = convert_todoist_color("red");
    let blue_color = convert_todoist_color("blue");
    // Colors should not be the same
    assert_ne!(red_color, blue_color, "Red and blue should be different colors");
}
