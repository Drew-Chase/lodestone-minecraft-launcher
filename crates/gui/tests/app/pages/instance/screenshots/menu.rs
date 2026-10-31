//! Tests for `pages/instance/screenshots/menu.rs`, kept out of the source file. They are compiled as the
//! `tests` child module of `menu`.

use super::*;

#[test]
fn the_menu_lists_copy_open_show_and_trash_with_a_separator_before_trash() {
    let menu = menu();
    assert_eq!(
        menu.actions,
        [Some(Action::Copy), Some(Action::Open), Some(Action::Reveal), None, Some(Action::Delete),]
    );
    assert_eq!(menu.items.len(), menu.actions.len());
    assert!(matches!(menu.items[3], MenuItem::Separator));
    assert!(matches!(menu.items[4], MenuItem::Action { danger: true, .. }), "trash is the danger row");
}
