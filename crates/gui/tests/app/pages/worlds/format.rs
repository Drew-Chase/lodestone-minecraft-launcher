//! Tests for `pages/worlds/format.rs`, kept out of the source file. They are compiled as the `tests` child module
//! of `format`.

use chrono::{Local, TimeZone};

use super::*;

fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single()
        .expect("a valid local time")
}

#[test]
fn recent_saves_count_in_minutes_and_hours() {
    let now = at(2026, 11, 2, 12, 0);

    assert_eq!(format_ago(now, now), "Just now");
    assert_eq!(format_ago(at(2026, 11, 2, 11, 30), now), "30m ago");
    assert_eq!(format_ago(at(2026, 11, 2, 10, 0), now), "2h ago");
}

#[test]
fn the_day_before_is_yesterday_and_then_days_and_weeks_follow() {
    let now = at(2026, 11, 2, 12, 0);

    assert_eq!(format_ago(at(2026, 11, 1, 9, 0), now), "Yesterday", "27 hours ago, on the day before");
    assert_eq!(format_ago(at(2026, 10, 30, 12, 0), now), "3d ago");
    assert_eq!(format_ago(at(2026, 10, 19, 12, 0), now), "2w ago");
}

#[test]
fn a_save_under_a_day_old_is_in_hours_even_across_midnight() {
    let now = at(2026, 11, 2, 0, 30);

    assert_eq!(format_ago(at(2026, 11, 1, 23, 30), now), "1h ago");
    assert_eq!(format_ago(at(2026, 11, 1, 20, 0), now), "4h ago");
}

#[test]
fn older_saves_show_their_date() {
    let now = at(2026, 11, 2, 12, 0);

    assert_eq!(format_ago(at(2026, 8, 1, 9, 0), now), "Aug 1, 2026");
}
