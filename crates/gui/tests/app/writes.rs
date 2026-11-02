//! Tests for `writes.rs`, kept out of the source file. They compile as its `tests` child module.

use super::*;

#[test]
fn the_first_save_of_a_key_starts_at_once() {
    let mut writer = KeyedWriter::default();
    assert_eq!(writer.request("a", 1), Some(1));
}

#[test]
fn a_save_during_a_write_waits_and_only_the_newest_is_written_after_it() {
    let mut writer = KeyedWriter::default();
    assert_eq!(writer.request("a", 1), Some(1));
    assert_eq!(writer.request("a", 2), None);
    assert_eq!(writer.request("a", 3), None);
    assert_eq!(writer.complete(&"a"), Some(3), "the snapshot that arrived last is written");
    assert_eq!(writer.complete(&"a"), None, "nothing waited behind the second write");
    assert_eq!(writer.request("a", 4), Some(4), "the key is free again");
}

#[test]
fn keys_do_not_wait_on_each_other() {
    let mut writer = KeyedWriter::default();
    assert_eq!(writer.request("a", 1), Some(1));
    assert_eq!(writer.request("b", 2), Some(2));
}
