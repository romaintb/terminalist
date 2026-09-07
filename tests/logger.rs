use terminalist::logger;

#[test]
fn test_log_file_path() {
    // Test that we can get the log file path
    let path = logger::get_log_file_path();
    assert!(path.is_ok());
    let path = path.unwrap();
    assert!(path.to_string_lossy().contains("terminalist.log"));
}
