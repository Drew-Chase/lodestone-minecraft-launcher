use iced::widget::text_editor::Position;

use super::*;

fn snapshot(text: &str, revision: u64) -> Snapshot {
    Snapshot {
        text: text.to_string(),
        cursor: Cursor {
            position: Position { line: 0, column: text.len() },
            selection: None,
        },
        revision,
    }
}

#[test]
fn letters_typed_close_together_are_one_step() {
    let mut history = History::default();
    let start = Instant::now();
    assert!(history.begins_step(Kind::Typing, start), "the first letter starts a step");
    assert!(!history.begins_step(Kind::Typing, start + Duration::from_millis(200)));
    assert!(
        !history.begins_step(Kind::Typing, start + Duration::from_millis(900)),
        "each letter is timed from the one before"
    );
    assert!(
        history.begins_step(Kind::Typing, start + Duration::from_secs(5)),
        "a pause starts a new step"
    );
}

#[test]
fn a_change_of_kind_or_a_move_of_the_caret_starts_a_new_step() {
    let mut history = History::default();
    let now = Instant::now();
    assert!(history.begins_step(Kind::Typing, now));
    assert!(history.begins_step(Kind::Deleting, now), "deleting is not typing");
    assert!(!history.begins_step(Kind::Deleting, now));
    history.break_step();
    assert!(history.begins_step(Kind::Deleting, now), "the caret moved in between");
}

#[test]
fn a_paste_or_a_new_line_is_always_a_step_of_its_own() {
    let mut history = History::default();
    let now = Instant::now();
    assert!(history.begins_step(Kind::Other, now));
    assert!(history.begins_step(Kind::Other, now));
    assert!(history.begins_step(Kind::Typing, now));
}

#[test]
fn undo_steps_back_through_the_text_and_redo_forward_again() {
    let mut history = History::default();
    history.push(snapshot("", 0));
    history.push(snapshot("a", 1));
    assert_eq!(history.undo(snapshot("ab", 2)), Some(snapshot("a", 1)));
    assert_eq!(history.undo(snapshot("a", 1)), Some(snapshot("", 0)));
    assert_eq!(history.undo(snapshot("", 0)), None, "nothing is left to undo");
    assert_eq!(history.redo(snapshot("", 0)), Some(snapshot("a", 1)));
    assert_eq!(history.redo(snapshot("a", 1)), Some(snapshot("ab", 2)));
    assert_eq!(history.redo(snapshot("ab", 2)), None);
}

#[test]
fn a_new_edit_after_an_undo_drops_what_could_be_redone() {
    let mut history = History::default();
    history.push(snapshot("", 0));
    let _ = history.undo(snapshot("a", 1));
    history.push(snapshot("", 0));
    assert_eq!(history.redo(snapshot("b", 2)), None);
}

#[test]
fn an_undo_ends_the_step_being_typed() {
    let mut history = History::default();
    let now = Instant::now();
    assert!(history.begins_step(Kind::Typing, now));
    history.push(snapshot("", 0));
    let _ = history.undo(snapshot("a", 1));
    assert!(history.begins_step(Kind::Typing, now), "typing after an undo is a new step");
}

#[test]
fn the_oldest_steps_go_when_the_history_holds_too_much_text() {
    let mut history = History::default();
    let big = "x".repeat(MAX_BYTES / 2 + 1);
    for revision in 0..4 {
        history.push(snapshot(&big, revision));
    }
    assert_eq!(history.undo.len(), 1, "two of these are over the cap, so only the newest stays");
    assert_eq!(history.undo[0].revision, 3);

    let mut history = History::default();
    history.push(snapshot(&"x".repeat(MAX_BYTES + 1), 0));
    assert_eq!(history.undo.len(), 1, "a single step over the cap is still kept");
}
