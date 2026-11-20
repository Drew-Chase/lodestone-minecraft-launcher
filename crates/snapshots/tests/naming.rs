use super::*;
use chrono::{NaiveDate, TimeDelta};

fn when() -> NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 11, 20)
        .expect("a date")
        .and_hms_opt(14, 5, 33)
        .expect("a time")
}

#[test]
fn a_stem_is_the_time_then_the_kind() {
    assert_eq!(stem(when(), Kind::UpdateAll, 1), "2026-11-20_14-05-33_update-all");
    assert_eq!(stem(when(), Kind::Manual, 1), "2026-11-20_14-05-33_manual");
    assert_eq!(stem(when(), Kind::Rollback, 1), "2026-11-20_14-05-33_rollback");
}

#[test]
fn a_later_copy_in_the_same_second_takes_a_counter() {
    assert_eq!(stem(when(), Kind::Manual, 2), "2026-11-20_14-05-33_manual-2");
}

#[test]
fn stems_read_back_to_the_name_they_came_from() {
    for kind in Kind::ALL {
        for copy in [1, 2, 12] {
            let name = Name { when: when(), copy, kind };
            assert_eq!(parse_stem(&name.stem()), Some(name));
        }
    }
}

#[test]
fn files_read_back_from_either_of_their_extensions() {
    let name = Name {
        when: when(),
        copy: 1,
        kind: Kind::UpdateAll,
    };
    assert_eq!(parse_file("2026-11-20_14-05-33_update-all.json.xz"), Some(name));
    assert_eq!(parse_file("2026-11-20_14-05-33_update-all.zip"), Some(name));
}

#[test]
fn files_that_are_not_snapshots_are_not_read() {
    assert_eq!(parse_file("2026-11-20_14-05-33_update-all.json.xz.partial"), None);
    assert_eq!(parse_file("2026-11-20_14-05-33_update-all.json"), None);
    assert_eq!(parse_file("notes.json.xz"), None);
    assert_eq!(parse_file("2026-11-20_14-05-33_other.zip"), None);
    assert_eq!(parse_file("2026-13-20_14-05-33_manual.zip"), None, "month 13 is not a time");
    assert_eq!(parse_file("2026-11-20_14-05-33_update-all-x.zip"), None);
    assert_eq!(parse_file("2026-11-20_14-05-33_manual-1.zip"), None, "the first copy is never numbered");
}

#[test]
fn names_order_by_time_then_copy() {
    let earlier = Name {
        when: when() - TimeDelta::hours(1),
        copy: 1,
        kind: Kind::Manual,
    };
    let later = Name {
        when: when(),
        copy: 1,
        kind: Kind::Manual,
    };
    let later_copy = Name { copy: 2, ..later };
    let mut names = vec![earlier, later, later_copy];
    names.sort();
    assert_eq!(names, vec![earlier, later, later_copy]);
}
