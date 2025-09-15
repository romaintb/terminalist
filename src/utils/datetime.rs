//! Date and time utility functions
//!
//! This module provides functions for date manipulation and human-readable formatting,
//! similar to how Todoist displays dates (e.g., "yesterday", "today", "tomorrow").

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Weekday};

/// Format a NaiveDate to YYYY-MM-DD string
pub fn format_ymd(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Calculate the next occurrence of a target weekday from a given date
pub fn next_weekday(from: NaiveDate, target: Weekday) -> NaiveDate {
    let from_w = from.weekday().num_days_from_monday() as i64;
    let tgt_w = target.num_days_from_monday() as i64;
    let mut delta = (7 + tgt_w - from_w) % 7;
    if delta == 0 {
        delta = 7;
    }
    from + Duration::days(delta)
}

/// Format a date string in Todoist-style human-readable format
///
/// # Arguments
/// * `date_str` - Date string in YYYY-MM-DD format
///
/// # Returns
/// * `String` - Human-readable date format
pub fn format_human_date(date_str: &str) -> String {
    // Parse the input date string
    let input_date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        Ok(date) => date,
        Err(_) => return date_str.to_string(), // Return original if parsing fails
    };

    // Get current local date
    let now = Local::now();
    let today = now.date_naive();

    // Calculate the difference in days
    let days_diff = (input_date - today).num_days();

    match days_diff {
        -1 => "yesterday".to_string(),
        0 => "today".to_string(),
        1 => "tomorrow".to_string(),
        diff if diff > 1 && diff <= 7 => {
            // Within the next week - show day name
            let weekday = input_date.weekday();
            format!("next {}", weekday_name(weekday))
        }
        diff if (-7..-1).contains(&diff) => {
            // Within the past week - show day name
            let weekday = input_date.weekday();
            format!("last {}", weekday_name(weekday))
        }
        diff if diff > 7 && diff <= 30 => {
            // Within the next month - show "in X days"
            format!("in {} days", diff)
        }
        diff if (-30..-7).contains(&diff) => {
            // Within the past month - show "X days ago"
            format!("{} days ago", -diff)
        }
        _ => {
            // For dates further out, show the actual date
            // Format as "Jan 15" or "Jan 15, 2025" if different year
            let current_year = today.year();
            let input_year = input_date.year();

            if input_year == current_year {
                input_date.format("%b %d").to_string()
            } else {
                input_date.format("%b %d, %Y").to_string()
            }
        }
    }
}

/// Format a datetime string in Todoist-style human-readable format
///
/// # Arguments
/// * `datetime_str` - DateTime string in various formats (RFC3339, ISO 8601, etc.)
///
/// # Returns
/// * `String` - Human-readable datetime format
pub fn format_human_datetime(datetime_str: &str) -> String {
    // Try multiple datetime parsing strategies
    let parsed_dt = if let Ok(dt) = DateTime::parse_from_rfc3339(datetime_str) {
        // RFC3339 with timezone (e.g., "2025-01-15T14:30:00Z")
        Some(dt.with_timezone(&Local))
    } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%dT%H:%M:%S") {
        // ISO 8601 without timezone (e.g., "2025-01-15T14:30:00")
        Some(
            Local
                .from_local_datetime(&dt)
                .single()
                .unwrap_or_else(|| Local.from_utc_datetime(&dt)),
        )
    } else if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S") {
        // Space-separated format (e.g., "2025-01-15 14:30:00")
        Some(
            Local
                .from_local_datetime(&dt)
                .single()
                .unwrap_or_else(|| Local.from_utc_datetime(&dt)),
        )
    } else {
        None
    };

    if let Some(local_dt) = parsed_dt {
        let date_str = local_dt.format("%Y-%m-%d").to_string();
        let time_str = local_dt.format("%H:%M").to_string();

        let human_date = format_human_date(&date_str);

        // Always show time for datetime strings
        format!("{} at {}", human_date, time_str)
    } else {
        // Fallback to date-only parsing
        format_human_date(datetime_str)
    }
}

/// Get a human-readable weekday name
fn weekday_name(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "Monday",
        Weekday::Tue => "Tuesday",
        Weekday::Wed => "Wednesday",
        Weekday::Thu => "Thursday",
        Weekday::Fri => "Friday",
        Weekday::Sat => "Saturday",
        Weekday::Sun => "Sunday",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Local};

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
}
