use super::*;

#[test]
fn a_pointer_over_content_the_view_shows_idle_reports_an_enter() {
    assert_eq!(transition(true, false), Some(true));
}

#[test]
fn a_pointer_off_content_the_view_shows_hovered_reports_an_exit() {
    assert_eq!(transition(false, true), Some(false));
}

#[test]
fn a_view_that_agrees_with_the_pointer_reports_nothing() {
    assert_eq!(transition(true, true), None, "still over it");
    assert_eq!(transition(false, false), None, "still away from it");
}

#[test]
fn a_card_the_page_cleared_under_a_still_pointer_enters_again() {
    // A scroll clears the page's hover while the pointer stays over the card: the next event reports the enter.
    assert_eq!(transition(true, false), Some(true));
}
