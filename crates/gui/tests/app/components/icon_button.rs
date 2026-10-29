//! Tests for `components/icon_button.rs`. They live here rather than inline and
//! are compiled as the `tests` child module of `icon_button`.

use iced::advanced::widget::Tree;
use iced::widget::Space;

use super::*;

/// A tree slot left by a stateless widget, such as a spacer or a container.
fn stateless_slot() -> Tree {
    let spacer: Element<'static, (), Theme, BaseRenderer> = Space::new().into();
    Tree::new(&spacer)
}

/// A button must claim a slot that held a stateless widget. Otherwise its
/// first `update` downcasts a `State::None` and the app panics.
#[test]
fn a_button_placed_over_a_stateless_slot_gets_its_own_state() {
    let mut slot = stateless_slot();
    assert!(matches!(slot.state, tree::State::None));

    let button: Element<'static, (), Theme, BaseRenderer> = IconButton::new("").on_press(()).into();
    slot.diff(&button);

    assert!(
        matches!(slot.state, tree::State::Some(_)),
        "the button must replace the stateless slot, not inherit its empty state"
    );
}

#[test]
fn a_redrawn_button_keeps_its_hover_state() {
    let button: Element<'static, (), Theme, BaseRenderer> = IconButton::new("").on_press(()).into();
    let mut slot = Tree::new(&button);
    slot.state.downcast_mut::<IconButtonState>().is_hovered = true;

    slot.diff(&button);

    assert!(slot.state.downcast_ref::<IconButtonState>().is_hovered);
}
