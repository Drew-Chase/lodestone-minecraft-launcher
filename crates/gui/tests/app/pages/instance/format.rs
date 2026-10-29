//! Tests for `pages/instance/format.rs`, kept out of the source file.

use super::*;

#[test]
fn small_byte_counts_keep_one_decimal() {
    assert_eq!(format_bytes(2_202_009), "2.1 MB");
    assert_eq!(format_bytes(1_503_238_554), "1.4 GB");
}

#[test]
fn larger_byte_counts_round_to_whole_units() {
    assert_eq!(format_bytes(432_013_312), "412 MB");
    assert_eq!(format_bytes(297_795_584), "284 MB");
}

#[test]
fn playtime_matches_the_overview_reference() {
    assert_eq!(format_playtime(24 * 3600 + 12 * 60), "24h 12m");
    assert_eq!(format_playtime(12 * 60), "12 min");
    assert_eq!(format_playtime(3600), "1h");
    assert_eq!(format_playtime(0), "—");
}

#[test]
fn the_clock_pads_minutes_like_the_worlds_reference() {
    assert_eq!(format_clock(9 * 3600 + 4 * 60), "9h 04m");
    assert_eq!(format_clock(48 * 3600 + 12 * 60), "48h 12m");
    assert_eq!(format_clock(45 * 60), "45m");
}
