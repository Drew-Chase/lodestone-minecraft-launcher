use super::*;

#[test]
fn a_card_menu_lists_play_edit_reveal_a_separator_then_delete() {
    let menu = menu(false, true);
    assert_eq!(menu.items.len(), 5);
    assert!(matches!(menu.items[3], MenuItem::Separator));
    assert!(matches!(menu.items[4], MenuItem::Action { danger: true, .. }), "delete is the red row");
    assert_eq!(
        menu.actions,
        vec![Some(Action::Play), Some(Action::Edit), Some(Action::Reveal), None, Some(Action::Delete)]
    );
}

#[test]
fn the_instance_page_leaves_reveal_out_for_a_local_instance() {
    let menu = menu(false, false);
    assert_eq!(menu.items.len(), 4);
    assert_eq!(menu.actions, vec![Some(Action::Play), Some(Action::Edit), None, Some(Action::Delete)]);
}

#[test]
fn an_install_that_has_not_finished_offers_only_cancelling_it() {
    let menu = menu(true, true);
    assert_eq!(menu.actions, vec![Some(Action::CancelInstall)]);
    assert!(matches!(menu.items[0], MenuItem::Action { danger: true, .. }));
}

#[test]
fn every_action_row_has_an_action_and_every_separator_has_none() {
    let menu = menu(false, true);
    assert_eq!(menu.items.len(), menu.actions.len());
    for (row, action) in menu.items.iter().zip(&menu.actions) {
        assert_eq!(action.is_some(), !matches!(row, MenuItem::Separator));
    }
}
