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

