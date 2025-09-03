//! Date utility functions

use chrono::{Datelike, Duration, NaiveDate, Weekday};

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Weekday;

    #[test]
    fn test_format_ymd() {
        let date = NaiveDate::from_ymd_opt(2023, 12, 25).unwrap();
        assert_eq!(format_ymd(date), "2023-12-25");
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
}