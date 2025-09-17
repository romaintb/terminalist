use std::fs;
use std::io::Write;
use terminalist::logger::Logger;

#[test]
fn test_config_based_logging_disabled() {
    // Test with logging disabled
    let logger = Logger::from_config(false).unwrap();
    assert!(!logger.is_enabled());
    assert!(!logger.has_file_writer());

    logger.log("Test message".to_string());
    let logs = logger.get_logs();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("Test message"));
}

#[test]
fn test_config_based_logging_enabled() {
    // Test with logging enabled
    let logger = Logger::from_config(true).unwrap();
    assert!(logger.is_enabled());
    assert!(logger.has_file_writer());

    logger.log("Test message with file".to_string());

    // Check in-memory logs (for UI display with "G" key)
    let logs = logger.get_logs();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("Test message with file"));

    // Test that the file writer exists and works by forcing a flush and checking the file
    if let Some(ref writer_arc) = logger.file_writer() {
        // Force flush the buffered writer
        if let Ok(mut writer) = writer_arc.lock() {
            let _ = writer.flush();
        }

        // Check if log file was created at the expected path
        let log_path = Logger::get_log_file_path().unwrap();
        if log_path.exists() {
            let file_content = fs::read_to_string(&log_path).unwrap_or_default();
            assert!(file_content.contains("Test message with file"));
            // Clean up test file
            let _ = fs::remove_file(&log_path);
        }
    }
}
