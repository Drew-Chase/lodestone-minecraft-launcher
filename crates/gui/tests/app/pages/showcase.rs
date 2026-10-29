use super::*;

#[test]
fn progress_cycles_through_the_whole_range() {
    let mut showcase = Showcase::default();
    for _ in 0..6 {
        showcase.update(Message::SimulateDownload);
    }
    // 6 × 0.17 wraps just past 1.0 — always clamped into range.
    assert!((0.0..1.0).contains(&showcase.progress));
}

#[test]
fn toggles_flip() {
    let mut showcase = Showcase::default();
    showcase.update(Message::ToggleAuto(true));
    assert!(showcase.toggle_auto);
    showcase.update(Message::ToggleAuto(false));
    assert!(!showcase.toggle_auto);
}

#[test]
fn dropdown_maps_item_indices() {
    let mut showcase = Showcase::default();
    showcase.update(Message::InstanceAction(0));
    assert_eq!(showcase.last_action, Some("Duplicate"));
    showcase.update(Message::InstanceAction(2));
    assert_eq!(showcase.last_action, Some("Delete"));
    showcase.update(Message::InstanceAction(1)); // separator — ignored
    assert_eq!(showcase.last_action, None);
}
