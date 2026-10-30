use super::*;

#[test]
fn a_window_never_runs_past_the_last_full_page() {
    assert_eq!(window_start(0, 7, 6), 0);
    assert_eq!(window_start(5, 7, 6), 1, "an offset past the end settles on the last page");
    assert_eq!(window_start(3, 4, 6), 0, "fewer items than a page show from the start");
}

#[test]
fn pages_step_forward_and_back_and_hold_at_the_ends() {
    assert_eq!(stepped(0, 7, 6, Step::Forward), 1);
    assert_eq!(stepped(1, 7, 6, Step::Forward), 1, "nothing lies past the last page");
    assert_eq!(stepped(1, 7, 6, Step::Back), 0);
    assert_eq!(stepped(0, 7, 6, Step::Back), 0);
    assert_eq!(stepped(0, 24, 6, Step::Forward), 6, "a full page moves a full page");
}

#[test]
fn revealing_brings_the_chosen_item_onto_the_page() {
    assert_eq!(revealing(0, 7, 6), 0);
    assert_eq!(revealing(6, 7, 6), 1, "the seventh item is only on the second page");
    assert_eq!(revealing(7, 8, 6), 2, "the last page is always a full page");
    assert!((revealing(7, 8, 6)..revealing(7, 8, 6) + 6).contains(&7));
}

#[test]
fn buttons_are_live_only_when_there_is_more_to_show() {
    assert!(can_step(0, 7, 6, Step::Forward));
    assert!(!can_step(1, 7, 6, Step::Forward));
    assert!(!can_step(0, 7, 6, Step::Back));
    assert!(can_step(1, 7, 6, Step::Back));
    assert!(!can_step(0, 4, 6, Step::Forward), "a short row has nothing to page to");
}
