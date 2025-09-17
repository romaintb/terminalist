use terminalist::utils::datetime::*;
use chrono::{Duration, Local, NaiveDate, Weekday};

#[test]
fn test_format_ymd() {
    let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
    assert_eq!(format_ymd(date), "2025-01-15");
}

#[test]
fn test_next_weekday() {
    let monday = NaiveDate::from_ymd_opt(2025, 1, 13).unwrap(); // Monday
    let friday = next_weekday(monday, Weekday::Fri);
    assert_eq!(friday, NaiveDate::from_ymd_opt(2025, 1, 17).unwrap());
}

#[test]
fn test_next_weekday_monday() {
    let friday = NaiveDate::from_ymd_opt(2023, 12, 22).unwrap(); // Friday
    let next_monday = next_weekday(friday, Weekday::Mon);
    let expected = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap(); // Next Monday
    assert_eq!(next_monday, expected);
}

#[test]
fn test_next_weekday_same_day() {
    let monday = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap(); // Monday
    let next_monday = next_weekday(monday, Weekday::Mon);
    let expected = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(); // Next Monday (7 days later)
    assert_eq!(next_monday, expected);
}

#[test]
fn test_format_human_date_today() {
    let today = Local::now().format("%Y-%m-%d").to_string();
    assert_eq!(format_human_date(&today), "today");
}

#[test]
fn test_format_human_date_tomorrow() {
    let tomorrow = (Local::now() + Duration::days(1)).format("%Y-%m-%d").to_string();
    assert_eq!(format_human_date(&tomorrow), "tomorrow");
}

#[test]
fn test_format_human_date_yesterday() {
    let yesterday = (Local::now() - Duration::days(1)).format("%Y-%m-%d").to_string();
    assert_eq!(format_human_date(&yesterday), "yesterday");
}

#[test]
fn test_format_human_datetime_iso_format() {
    // Test the specific format from the user's example
    let datetime_str = "2025-09-16T09:00:00";
    let formatted = format_human_datetime(datetime_str);

    // Should contain time information and be human-readable
    assert!(formatted.contains("at"));
    assert!(formatted.contains("09:00"));
}