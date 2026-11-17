use super::*;

fn letter(text: &str) -> KeyboardKey {
    KeyboardKey::Character(text.into())
}

#[test]
fn the_clipboard_keys_need_the_command_key() {
    assert_eq!(key_of(&letter("c"), Modifiers::COMMAND, false), Some(Key::Copy));
    assert_eq!(key_of(&letter("X"), Modifiers::COMMAND | Modifiers::SHIFT, false), Some(Key::Cut));
    assert_eq!(key_of(&letter("v"), Modifiers::COMMAND, false), Some(Key::Paste));
    assert_eq!(key_of(&letter("a"), Modifiers::COMMAND, false), Some(Key::SelectAll));
    assert_eq!(key_of(&letter("c"), Modifiers::empty(), false), None);
    assert_eq!(key_of(&letter("q"), Modifiers::COMMAND, false), None);
}

#[test]
fn the_arrows_step_the_selection_and_shift_extends_it() {
    let down = KeyboardKey::Named(Named::ArrowDown);
    assert_eq!(key_of(&down, Modifiers::empty(), false), Some(Key::Step { delta: 1, extend: false }));
    assert_eq!(key_of(&down, Modifiers::SHIFT, false), Some(Key::Step { delta: 1, extend: true }));
    let up = KeyboardKey::Named(Named::ArrowUp);
    assert_eq!(key_of(&up, Modifiers::empty(), false), Some(Key::Step { delta: -1, extend: false }));
    assert_eq!(key_of(&up, Modifiers::ALT, false), Some(Key::Up), "Alt with Up goes to the folder above");
}

#[test]
fn the_named_keys_map_to_their_actions() {
    let none = Modifiers::empty();
    assert_eq!(key_of(&KeyboardKey::Named(Named::Delete), none, false), Some(Key::Delete));
    assert_eq!(key_of(&KeyboardKey::Named(Named::F2), none, false), Some(Key::Rename));
    assert_eq!(key_of(&KeyboardKey::Named(Named::Enter), none, false), Some(Key::Open));
    assert_eq!(key_of(&KeyboardKey::Named(Named::Backspace), none, false), Some(Key::Up));
    assert_eq!(
        key_of(&KeyboardKey::Named(Named::Escape), none, false),
        Some(Key::Escape { taken: false })
    );
}

#[test]
fn a_key_a_widget_took_is_left_alone_except_escape() {
    let none = Modifiers::empty();
    assert_eq!(
        key_of(&letter("v"), Modifiers::COMMAND, true),
        None,
        "a paste into the rename field is not a file paste"
    );
    assert_eq!(key_of(&KeyboardKey::Named(Named::Delete), none, true), None);
    assert_eq!(key_of(&KeyboardKey::Named(Named::Escape), none, true), Some(Key::Escape { taken: true }));
}

#[test]
fn the_modifiers_a_click_reads_are_command_and_shift() {
    assert_eq!(held(Modifiers::COMMAND | Modifiers::SHIFT), Held { command: true, shift: true });
    assert_eq!(held(Modifiers::ALT), Held::default());
}
