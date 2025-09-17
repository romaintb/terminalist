use terminalist::ui::core::event_handler::EventType;

#[test]
fn test_event_type_enum_exists() {
    // Test that EventType enum is accessible and has a valid size
    let event_size = std::mem::size_of::<EventType>();
    // EventType enum should have a non-zero size
    assert!(event_size > 0, "EventType enum should have a non-zero size");
}
