use chrono::{DateTime, Local, NaiveDate, Weekday};
use terminalist::utils::datetime::*;

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
    let today = format_today();
    assert_eq!(format_human_date(&today), "today");
}

#[test]
fn test_format_human_date_tomorrow() {
    let tomorrow = format_date_with_offset(1);
    assert_eq!(format_human_date(&tomorrow), "tomorrow");
}

#[test]
fn test_format_human_date_yesterday() {
    let yesterday = format_date_with_offset(-1);
    assert_eq!(format_human_date(&yesterday), "yesterday");
}

#[test]
fn test_format_human_date_within_week_shows_full_weekday_name() {
    let date = chrono::Local::now().date_naive() + chrono::Duration::days(3);
    let expected = format!("next {}", date.format("%A"));
    assert_eq!(format_human_date(&format_ymd(date)), expected);
    // Guard against abbreviated names sneaking back in ("next Sat" instead of "next Saturday")
    assert!(expected.len() > "next Sat".len());
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

#[test]
fn test_split_todoist_due_keeps_a_plain_date() {
    assert_eq!(split_todoist_due("2026-11-20", None), ("2026-11-20".to_string(), None));
}

/// Todoist puts a floating time inside `date`. The day alone must go to `due_date`, or the
/// Today and Upcoming views drop the task.
#[test]
fn test_split_todoist_due_moves_a_floating_time_out_of_the_date() {
    assert_eq!(
        split_todoist_due("2026-11-20T13:00:00", None),
        ("2026-11-20".to_string(), Some("2026-11-20T13:00:00".to_string()))
    );
}

#[test]
fn test_split_todoist_due_prefers_an_explicit_datetime() {
    let (day, time) = split_todoist_due("2026-11-20T13:00:00", Some("2026-11-20T13:00:00.000000"));
    assert_eq!(day, "2026-11-20");
    assert_eq!(time.as_deref(), Some("2026-11-20T13:00:00.000000"));
}

/// A trailing "Z" is a UTC instant, so the day must be the same instant seen from here, not
/// the date prefix of the string.
#[test]
fn test_split_todoist_due_takes_a_utc_instant_in_local_time() {
    let instant = "2026-11-20T23:30:00Z";
    let expected_day = DateTime::parse_from_rfc3339(instant)
        .expect("test instant parses")
        .with_timezone(&Local)
        .format("%Y-%m-%d")
        .to_string();

    let (day, time) = split_todoist_due(instant, None);

    assert_eq!(day, expected_day);
    assert_eq!(time.as_deref(), Some(instant));
}

/// The API leaves `datetime` empty on a timed task; that must not blank the time.
#[test]
fn test_split_todoist_due_treats_an_empty_datetime_as_absent() {
    assert_eq!(
        split_todoist_due("2026-11-20T13:00:00", Some("")),
        ("2026-11-20".to_string(), Some("2026-11-20T13:00:00".to_string()))
    );
}

#[test]
fn test_format_human_datetime_shows_the_time_with_or_without_fractions() {
    assert!(format_human_datetime("2026-11-20T13:00:00").ends_with("at 13:00"));
    assert!(format_human_datetime("2026-11-20T13:00:00.000000").ends_with("at 13:00"));
}
