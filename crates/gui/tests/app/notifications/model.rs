use super::*;
use chrono::{Local, TimeZone};

#[test]
fn every_kind_is_stored_under_a_name_it_reads_back_as() {
    for kind in Kind::ALL {
        assert_eq!(Kind::from_db(kind.to_db()), Some(kind), "{kind:?}");
    }
}

#[test]
fn a_stored_name_this_build_does_not_know_is_no_kind() {
    assert_eq!(Kind::from_db("made_up_later"), None);
}

#[test]
fn an_event_key_keeps_the_moment_to_the_millisecond() {
    let at = Local.timestamp_millis_opt(1_700_000_000_123).single().expect("a valid local time");
    assert_eq!(event_key("crash:abc", at), "crash:abc:1700000000123");
}
