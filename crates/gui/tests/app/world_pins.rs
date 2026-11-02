use super::*;

#[test]
fn rows_from_the_database_mark_their_worlds_pinned() {
    let pins = WorldPins::from_rows(vec![("alpha".into(), "Hub".into()), ("beta".into(), "Hub".into())]);

    assert!(pins.contains("alpha", "Hub"));
    assert!(pins.contains("beta", "Hub"), "the same folder in another instance is its own world");
    assert!(!pins.contains("alpha", "Other"));
    assert!(!pins.contains("gamma", "Hub"));
}

#[test]
fn unpinning_clears_only_that_world() {
    let mut pins = WorldPins::from_rows(vec![("alpha".into(), "Hub".into()), ("alpha".into(), "Spawn".into())]);

    pins.set("alpha", "Hub", false);

    assert!(!pins.contains("alpha", "Hub"));
    assert!(pins.contains("alpha", "Spawn"));
}

#[test]
fn forgetting_an_instance_drops_its_pins_and_no_others() {
    let mut pins = WorldPins::from_rows(vec![("alpha".into(), "Hub".into()), ("beta".into(), "Hub".into())]);

    pins.forget_instance("alpha");

    assert!(!pins.contains("alpha", "Hub"));
    assert!(pins.contains("beta", "Hub"));
}

#[test]
fn pinning_twice_keeps_one_pin() {
    let mut pins = WorldPins::default();

    pins.set("alpha", "Hub", true);
    pins.set("alpha", "Hub", true);
    pins.set("alpha", "Hub", false);

    assert_eq!(pins, WorldPins::default(), "one unpin clears the pin, however many pins were made");
}
