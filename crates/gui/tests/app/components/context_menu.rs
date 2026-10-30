use super::*;
use iced::widget::Space;

fn one_row() -> Vec<MenuItem> {
    vec![MenuItem::Action {
        label: String::from("Open"),
        icon: None,
        danger: false,
    }]
}

#[test]
fn a_menu_is_stale_only_for_another_key() {
    assert!(is_stale(Some("sky"), "aether"), "another target's menu must drop");
    assert!(is_stale(None, "aether"), "a menu that never opened has no target");
    assert!(!is_stale(Some("aether"), "aether"));
}

#[test]
fn a_menu_opens_only_with_rows_and_a_handler() {
    let bare = ContextMenu::<()>::new(Space::new(), "aether");
    assert!(!bare.interactive(), "no rows and no handler");

    let rows_only = ContextMenu::<()>::new(Space::new(), "aether").items(one_row());
    assert!(!rows_only.interactive(), "rows need a handler to report a pick");

    let ready = ContextMenu::<()>::new(Space::new(), "aether").items(one_row()).on_select(|_| ());
    assert!(ready.interactive());
}
