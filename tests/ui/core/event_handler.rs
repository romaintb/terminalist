use terminalist::ui::core::event_handler::EventType;

#[test]
fn test_event_type_enum_exists() {
    // Test that EventType enum is accessible
    let _event_size = std::mem::size_of::<EventType>();
    assert!(true, "EventType enum should be accessible");
}