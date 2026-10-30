use std::time::{Duration, Instant};

use iced::keyboard::key::{Code, Named, Physical};
use iced::keyboard::{Key, Modifiers};

use super::*;

/// A query holding `text`, with the caret at its end.
fn query_of(text: &str) -> Query {
    let mut query = Query::default();
    query.insert(text);
    query
}

#[test]
fn typing_lands_at_the_caret() {
    let mut query = query_of("ab");
    query.left();
    assert!(query.insert("X"));
    assert_eq!(query.text(), "aXb");
    assert_eq!(query.before_caret(), "aX");
}

#[test]
fn typing_with_the_caret_at_the_end_appends() {
    let mut query = query_of("ab");
    query.insert("cd");
    assert_eq!(query.text(), "abcd");
}

#[test]
fn control_characters_are_not_typed() {
    let mut query = Query::default();
    assert!(!query.insert("\u{7}"));
    assert!(!query.insert("\n"));
    assert!(query.is_empty());
}

#[test]
fn left_and_right_move_one_character() {
    let mut query = query_of("ab");
    query.left();
    assert_eq!(query.before_caret(), "a");
    query.right();
    assert_eq!(query.before_caret(), "ab");
}

#[test]
fn the_arrows_stop_at_the_ends() {
    let mut query = query_of("ab");
    query.right();
    assert_eq!(query.before_caret(), "ab");
    query.home();
    query.left();
    assert_eq!(query.before_caret(), "");
}

#[test]
fn home_and_end_jump_to_the_ends() {
    let mut query = query_of("abc");
    query.home();
    assert_eq!(query.before_caret(), "");
    query.end();
    assert_eq!(query.before_caret(), "abc");
}

#[test]
fn the_caret_steps_over_whole_characters() {
    let mut query = query_of("aéb");
    query.left();
    assert_eq!(query.before_caret(), "aé");
    query.left();
    assert_eq!(query.before_caret(), "a");
    query.right();
    assert_eq!(query.before_caret(), "aé");
}

#[test]
fn backspace_removes_the_character_before_the_caret() {
    let mut query = query_of("abc");
    query.left();
    assert!(query.backspace());
    assert_eq!(query.text(), "ac");
    assert_eq!(query.before_caret(), "a");
}

#[test]
fn backspace_at_the_start_removes_nothing() {
    let mut query = query_of("abc");
    query.home();
    assert!(!query.backspace());
    assert_eq!(query.text(), "abc");
}

#[test]
fn delete_removes_the_character_after_the_caret() {
    let mut query = query_of("abc");
    query.home();
    query.right();
    assert!(query.delete());
    assert_eq!(query.text(), "ac");
    assert_eq!(query.before_caret(), "a");
}

#[test]
fn delete_at_the_end_removes_nothing() {
    let mut query = query_of("abc");
    assert!(!query.delete());
    assert_eq!(query.text(), "abc");
}

#[test]
fn editing_multibyte_text_keeps_it_whole() {
    let mut query = query_of("aé");
    assert!(query.backspace());
    assert_eq!(query.text(), "a");

    let mut query = query_of("é");
    query.home();
    assert!(query.delete());
    assert!(query.is_empty());
}

#[test]
fn clear_empties_the_field_and_returns_the_caret() {
    let mut query = query_of("abc");
    query.clear();
    assert!(query.is_empty());
    assert_eq!(query.before_caret(), "");
}

#[test]
fn the_caret_shows_for_half_of_each_blink() {
    let since = Instant::now();
    assert!(caret_visible(since, since));
    assert!(caret_visible(since, since + Duration::from_millis(499)));
    assert!(!caret_visible(since, since + Duration::from_millis(500)));
    assert!(!caret_visible(since, since + Duration::from_millis(999)));
    assert!(caret_visible(since, since + Duration::from_millis(1000)));
}

#[test]
fn a_moved_caret_shows_again_at_once() {
    let since = Instant::now();
    let moved = since + Duration::from_millis(700);
    assert!(!caret_visible(since, moved));
    assert!(caret_visible(moved, moved));
}

#[test]
fn the_next_blink_lands_on_the_next_flip() {
    let since = Instant::now();
    assert_eq!(next_blink(since, since), Duration::from_millis(500));
    assert_eq!(next_blink(since, since + Duration::from_millis(120)), Duration::from_millis(380));
    assert_eq!(next_blink(since, since + Duration::from_millis(500)), Duration::from_millis(500));
}

#[test]
fn a_key_types_its_own_text() {
    assert_eq!(
        typed_text(&Key::Character("é".into()), Physical::Code(Code::KeyE), Modifiers::default(), Some("é")),
        Some("é")
    );
}

#[test]
fn a_numpad_digit_without_text_types_its_character() {
    assert_eq!(
        typed_text(&Key::Character("8".into()), Physical::Code(Code::Numpad8), Modifiers::default(), None),
        Some("8")
    );
}

#[test]
fn keypad_digits_type_their_digit_with_numlock_off() {
    // With NumLock off the keypad sends navigation keys, but each digit key still types its digit.
    assert_eq!(
        typed_text(&Key::Named(Named::ArrowUp), Physical::Code(Code::Numpad8), Modifiers::default(), None),
        Some("8")
    );
    assert_eq!(
        typed_text(&Key::Named(Named::ArrowDown), Physical::Code(Code::Numpad2), Modifiers::default(), None),
        Some("2")
    );
    assert_eq!(
        typed_text(&Key::Named(Named::ArrowLeft), Physical::Code(Code::Numpad4), Modifiers::default(), None),
        Some("4")
    );
    assert_eq!(
        typed_text(&Key::Named(Named::Home), Physical::Code(Code::Numpad7), Modifiers::default(), None),
        Some("7")
    );
    assert_eq!(
        typed_text(&Key::Named(Named::Insert), Physical::Code(Code::Numpad0), Modifiers::default(), None),
        Some("0")
    );
}

#[test]
fn keypad_decimal_types_a_dot() {
    assert_eq!(
        typed_text(
            &Key::Named(Named::Delete),
            Physical::Code(Code::NumpadDecimal),
            Modifiers::default(),
            None
        ),
        Some(".")
    );
}

#[test]
fn shortcuts_and_named_keys_without_text_type_nothing() {
    assert_eq!(
        typed_text(&Key::Character("v".into()), Physical::Code(Code::KeyV), Modifiers::CTRL, Some("v")),
        None
    );
    assert_eq!(
        typed_text(&Key::Named(Named::ArrowLeft), Physical::Code(Code::ArrowLeft), Modifiers::default(), None),
        None
    );
    assert_eq!(
        typed_text(&Key::Named(Named::ArrowUp), Physical::Code(Code::Numpad8), Modifiers::CTRL, None),
        None
    );
    assert_eq!(
        typed_text(&Key::Named(Named::Delete), Physical::Code(Code::Delete), Modifiers::default(), None),
        None
    );
}

#[test]
fn control_keys_type_nothing_so_they_pass_through() {
    assert_eq!(
        typed_text(&Key::Named(Named::Tab), Physical::Code(Code::Tab), Modifiers::default(), Some("\t")),
        None
    );
}
