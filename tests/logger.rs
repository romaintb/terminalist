use terminalist::logger;

#[test]
fn test_memory_logs() {
    // Clear any existing logs
    logger::clear_memory_logs();

    // Test that we can get empty logs
    let logs = logger::get_memory_logs();
    assert!(logs.is_empty());
}

#[test]
fn test_log_file_path() {
    // Test that we can get the log file path
    let path = logger::get_log_file_path();
    assert!(path.is_ok());
    let path = path.unwrap();
    assert!(path.to_string_lossy().contains("terminalist.log"));
}
