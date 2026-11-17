use super::*;

const ORDER: [&str; 5] = ["config", "mods", "a.txt", "b.txt", "c.txt"];

const PLAIN: Held = Held {
    command: false,
    shift: false,
};
const COMMAND: Held = Held { command: true, shift: false };
const SHIFT: Held = Held { command: false, shift: true };

#[test]
fn a_plain_click_selects_one_row() {
    let mut selection = Selection::default();
    selection.click(&ORDER, "a.txt", PLAIN);
    selection.click(&ORDER, "mods", PLAIN);
    assert_eq!(selection.paths(), ["mods"]);
    assert_eq!(selection.single(), Some("mods"));
}

#[test]
fn a_command_click_adds_a_row_and_removes_it_again() {
    let mut selection = Selection::default();
    selection.click(&ORDER, "a.txt", PLAIN);
    selection.click(&ORDER, "c.txt", COMMAND);
    assert_eq!(selection.paths(), ["a.txt", "c.txt"]);
    assert_eq!(selection.single(), None, "two rows are not a single selection");
    selection.click(&ORDER, "a.txt", COMMAND);
    assert_eq!(selection.paths(), ["c.txt"]);
}

#[test]
fn a_shift_click_selects_the_range_from_the_row_picked_last() {
    let mut selection = Selection::default();
    selection.click(&ORDER, "mods", PLAIN);
    selection.click(&ORDER, "b.txt", SHIFT);
    assert_eq!(selection.paths(), ["a.txt", "b.txt", "mods"]);

    // The anchor stays, so a second Shift click replaces the range.
    selection.click(&ORDER, "config", SHIFT);
    assert_eq!(selection.paths(), ["config", "mods"]);
}

#[test]
fn a_shift_click_with_nothing_selected_picks_that_row() {
    let mut selection = Selection::default();
    selection.click(&ORDER, "b.txt", SHIFT);
    assert_eq!(selection.paths(), ["b.txt"]);
}

#[test]
fn select_all_takes_every_row_and_clear_takes_none() {
    let mut selection = Selection::default();
    selection.all(&ORDER);
    assert_eq!(selection.len(), 5);
    selection.clear();
    assert!(selection.is_empty());
}

#[test]
fn the_arrow_keys_move_one_row_and_stop_at_the_ends() {
    let mut selection = Selection::default();
    selection.step(&ORDER, 1, false);
    assert_eq!(selection.paths(), ["config"], "a step down from nothing starts at the top");
    selection.step(&ORDER, 1, false);
    assert_eq!(selection.paths(), ["mods"]);
    selection.step(&ORDER, -5, false);
    assert_eq!(selection.paths(), ["config"]);

    let mut selection = Selection::default();
    selection.step(&ORDER, -1, false);
    assert_eq!(selection.paths(), ["c.txt"], "a step up from nothing starts at the bottom");
    selection.step(&ORDER, 3, false);
    assert_eq!(selection.paths(), ["c.txt"]);
}

#[test]
fn shift_with_an_arrow_key_grows_the_range_from_the_anchor() {
    let mut selection = Selection::default();
    selection.click(&ORDER, "a.txt", PLAIN);
    selection.step(&ORDER, 1, true);
    selection.step(&ORDER, 1, true);
    assert_eq!(selection.paths(), ["a.txt", "b.txt", "c.txt"]);
    selection.step(&ORDER, -3, true);
    assert_eq!(selection.paths(), ["a.txt", "mods"], "the range turns round at the anchor");
    assert_eq!(selection.lead(), Some("mods"));
}

#[test]
fn rows_that_are_gone_leave_the_selection() {
    let mut selection = Selection::default();
    selection.all(&ORDER);
    selection.retain(|path| path != "a.txt" && path != "config");
    assert_eq!(selection.paths(), ["b.txt", "c.txt", "mods"]);
    // The anchor was `config`, which is gone, so a Shift click starts a fresh range.
    selection.click(&["mods", "b.txt", "c.txt"], "b.txt", SHIFT);
    assert_eq!(selection.paths(), ["b.txt"]);
}
