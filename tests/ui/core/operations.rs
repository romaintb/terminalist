//! The one piece of real logic in the operations module: turning a due-date shorthand
//! into a date. The rest of an Operation is data the type system carries for us.

use chrono::{Datelike, NaiveDate, Weekday};
use terminalist::ui::core::operations::Due;
use terminalist::utils::datetime;

fn parse(due: Due) -> NaiveDate {
    NaiveDate::parse_from_str(&due.date(), "%Y-%m-%d").unwrap_or_else(|e| panic!("{due:?} gave {:?}: {e}", due.date()))
}

#[test]
fn today_and_tomorrow_match_the_shared_date_helpers() {
    assert_eq!(Due::Today.date(), datetime::format_today());
    assert_eq!(Due::Tomorrow.date(), datetime::format_date_with_offset(1));
    assert_eq!(parse(Due::Tomorrow), parse(Due::Today).succ_opt().unwrap());
}

/// Next week means Monday and the weekend means Saturday. A swap between the two would
/// otherwise be invisible: both are plausible dates a few days out.
#[test]
fn next_week_lands_on_monday_and_the_weekend_on_saturday() {
    assert_eq!(parse(Due::NextWeek).weekday(), Weekday::Mon);
    assert_eq!(parse(Due::Weekend).weekday(), Weekday::Sat);
    assert!(parse(Due::NextWeek) >= parse(Due::Today));
    assert!(parse(Due::Weekend) >= parse(Due::Today));
}

/// Four shorthands, four distinct confirmation messages.
#[test]
fn every_due_shorthand_has_its_own_message() {
    let messages = [Due::Today, Due::Tomorrow, Due::NextWeek, Due::Weekend].map(Due::success);
    let mut unique = messages.to_vec();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), messages.len(), "duplicate message in {messages:?}");
}
