//! The Files tab's keys and file drops, read from the window's events while the tab is on screen.
//!
//! The keys are the ones a file manager answers: Ctrl+C, Ctrl+X and Ctrl+V, Ctrl+A, Delete, F2, Enter, Backspace, the
//! arrows and Escape. A key a widget already took is left alone, so typing in the rename field or the editor never
//! reaches the list.

use iced::event::Status;
use iced::keyboard::key::Named;
use iced::keyboard::{self, Key as KeyboardKey, Modifiers};
use iced::{Event, window};

use super::Message;
use super::selection::Held;

/// What a key asks of the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Copy,
    Cut,
    Paste,
    SelectAll,
    Delete,
    Rename,
    /// Opens the selected row.
    Open,
    /// Goes to the folder above.
    Up,
    /// Moves the selection by this many rows. `extend` keeps the range from the row it started at.
    Step {
        delta: isize,
        extend: bool,
    },
    /// Escape. `taken` is whether a widget took it first, as the rename field does to drop its focus.
    Escape {
        taken: bool,
    },
}

/// The message for a window event, when the list answers it.
pub fn event(event: Event, status: Status, _window: window::Id) -> Option<Message> {
    match event {
        Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => Some(Message::Held(held(modifiers))),
        Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => key_of(&key, modifiers, status != Status::Ignored).map(Message::Key),
        _ => None,
    }
}

/// The message for a file dragged over the window, leaving it, or dropped on it.
pub fn drop_event(event: Event, _status: Status, _window: window::Id) -> Option<Message> {
    match event {
        Event::Window(window::Event::FileHovered(_)) => Some(Message::FileHover(true)),
        Event::Window(window::Event::FilesHoveredLeft) => Some(Message::FileHover(false)),
        Event::Window(window::Event::FileDropped(path)) => Some(Message::Dropped(path)),
        _ => None,
    }
}

/// The modifier keys a click reads.
pub fn held(modifiers: Modifiers) -> Held {
    Held {
        command: modifiers.command(),
        shift: modifiers.shift(),
    }
}

/// What `key` asks of the list with `modifiers` held. `taken` is whether a widget took the key first, in which case
/// only Escape is passed on.
fn key_of(key: &KeyboardKey, modifiers: Modifiers, taken: bool) -> Option<Key> {
    if taken {
        return matches!(key.as_ref(), KeyboardKey::Named(Named::Escape)).then_some(Key::Escape { taken: true });
    }
    let command = modifiers.command();
    let step = |delta| Key::Step {
        delta,
        extend: modifiers.shift(),
    };
    let key = match key.as_ref() {
        KeyboardKey::Character(letter) if command && !modifiers.alt() => match letter.to_ascii_lowercase().as_str() {
            "c" => Key::Copy,
            "x" => Key::Cut,
            "v" => Key::Paste,
            "a" => Key::SelectAll,
            _ => return None,
        },
        KeyboardKey::Named(Named::ArrowUp) if modifiers.alt() => Key::Up,
        KeyboardKey::Named(Named::ArrowUp) => step(-1),
        KeyboardKey::Named(Named::ArrowDown) => step(1),
        KeyboardKey::Named(Named::PageUp) => step(-10),
        KeyboardKey::Named(Named::PageDown) => step(10),
        KeyboardKey::Named(Named::Home) => step(isize::MIN),
        KeyboardKey::Named(Named::End) => step(isize::MAX),
        KeyboardKey::Named(Named::Delete) => Key::Delete,
        KeyboardKey::Named(Named::F2) => Key::Rename,
        KeyboardKey::Named(Named::Enter) => Key::Open,
        KeyboardKey::Named(Named::Backspace) => Key::Up,
        KeyboardKey::Named(Named::Escape) => Key::Escape { taken: false },
        _ => return None,
    };
    Some(key)
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/browser/keys.rs"]
mod tests;
