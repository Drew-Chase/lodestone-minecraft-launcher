//! Tests for `retention`. They pass the time in, so nothing waits on a clock.

use super::*;

/// The keys a sweep drops, sorted, for comparisons that do not depend on hash order.
fn sorted(mut keys: Vec<&'static str>) -> Vec<&'static str> {
    keys.sort_unstable();
    keys
}

#[test]
fn an_entry_survives_up_to_the_idle_limit() {
    let start = Instant::now();
    let mut cache = Retained::default();
    cache.insert("a", 1, start);

    assert!(cache.sweep(start + IDLE).is_empty(), "exactly the idle limit still counts as fresh");
    assert!(cache.contains(&"a"));
}

#[test]
fn an_entry_goes_once_it_has_been_idle_longer_than_the_limit() {
    let start = Instant::now();
    let mut cache = Retained::default();
    cache.insert("a", 1, start);

    let dropped = cache.sweep(start + IDLE + Duration::from_millis(1));

    assert_eq!(dropped, vec!["a"]);
    assert!(cache.is_empty());
}

#[test]
fn a_touch_restarts_the_idle_clock() {
    let start = Instant::now();
    let mut cache = Retained::default();
    cache.insert("a", 1, start);
    cache.touch(&"a", start + Duration::from_secs(20));

    assert!(cache.sweep(start + Duration::from_secs(45)).is_empty(), "25 seconds idle since the touch");
    assert!(cache.contains(&"a"));
}

#[test]
fn reading_with_get_does_not_count_as_a_use() {
    let start = Instant::now();
    let mut cache = Retained::default();
    cache.insert("a", 1, start);
    assert_eq!(cache.get(&"a"), Some(&1));

    assert_eq!(cache.sweep(start + Duration::from_secs(40)), vec!["a"]);
}

#[test]
fn get_touch_counts_as_a_use() {
    let start = Instant::now();
    let mut cache = Retained::default();
    cache.insert("a", 1, start);
    assert_eq!(cache.get_touch(&"a", start + Duration::from_secs(25)), Some(&1));

    assert!(cache.sweep(start + Duration::from_secs(40)).is_empty());
}

#[test]
fn a_sweep_drops_only_the_idle_entries_and_names_them() {
    let start = Instant::now();
    let mut cache = Retained::default();
    cache.insert("old", 1, start);
    cache.insert("used", 2, start + Duration::from_secs(20));
    cache.insert("new", 3, start + Duration::from_secs(29));

    let dropped = cache.sweep(start + Duration::from_secs(40));

    assert_eq!(sorted(dropped), vec!["old"]);
    assert_eq!(cache.len(), 2);
    assert!(cache.contains(&"used") && cache.contains(&"new"));
}

#[test]
fn remove_takes_the_value_out() {
    let start = Instant::now();
    let mut cache = Retained::default();
    cache.insert("a", String::from("picture"), start);

    assert_eq!(cache.remove(&"a").as_deref(), Some("picture"));
    assert!(!cache.contains(&"a"));
    assert!(cache.remove(&"a").is_none());
}

#[test]
fn touch_on_a_missing_key_does_nothing() {
    let start = Instant::now();
    let mut cache: Retained<&str, u8> = Retained::default();
    cache.touch(&"missing", start);

    assert!(cache.is_empty());
}

#[test]
fn clear_and_iter_cover_every_entry() {
    let start = Instant::now();
    let mut cache = Retained::default();
    cache.insert("a", 1, start);
    cache.insert("b", 2, start);

    let mut seen: Vec<(&str, i32)> = cache.iter().map(|(key, value)| (*key, *value)).collect();
    seen.sort_unstable();
    assert_eq!(seen, vec![("a", 1), ("b", 2)]);

    cache.clear();
    assert!(cache.is_empty());
}
