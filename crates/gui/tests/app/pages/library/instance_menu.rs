use super::*;

#[test]
fn a_card_menu_lists_play_edit_lock_mod_updates_reveal_export_server_pack_a_separator_then_delete() {
    let menu = menu(false, true, false, false);
    assert_eq!(menu.items.len(), 9);
    assert!(matches!(menu.items[7], MenuItem::Separator));
    assert!(matches!(menu.items[8], MenuItem::Action { danger: true, .. }), "delete is the red row");
    assert_eq!(
        menu.actions,
        vec![
            Some(Action::Play),
            Some(Action::Edit),
            Some(Action::Lock),
            Some(Action::DisableModUpdates),
            Some(Action::Reveal),
            Some(Action::Export),
            Some(Action::ServerPack),
            None,
            Some(Action::Delete)
        ]
    );
}

#[test]
fn the_instance_page_leaves_reveal_out_for_a_local_instance() {
    let menu = menu(false, false, false, false);
    assert_eq!(menu.items.len(), 8);
    assert_eq!(
        menu.actions,
        vec![
            Some(Action::Play),
            Some(Action::Edit),
            Some(Action::Lock),
            Some(Action::DisableModUpdates),
            Some(Action::Export),
            Some(Action::ServerPack),
            None,
            Some(Action::Delete)
        ]
    );
}

#[test]
fn a_locked_instance_offers_to_unlock_it_and_an_unlocked_one_to_lock_it() {
    let locked = menu(false, true, true, false).actions;
    assert!(locked.contains(&Some(Action::Unlock)));
    assert!(!locked.contains(&Some(Action::Lock)));
    let unlocked = menu(false, true, false, false).actions;
    assert!(unlocked.contains(&Some(Action::Lock)));
    assert!(!unlocked.contains(&Some(Action::Unlock)));
}

#[test]
fn the_mod_update_row_turns_the_checks_off_while_they_are_on_and_on_while_they_are_off() {
    let on = menu(false, true, false, false).actions;
    assert!(on.contains(&Some(Action::DisableModUpdates)));
    assert!(!on.contains(&Some(Action::EnableModUpdates)));
    let off = menu(false, true, false, true).actions;
    assert!(off.contains(&Some(Action::EnableModUpdates)));
    assert!(!off.contains(&Some(Action::DisableModUpdates)));
}

#[test]
fn an_install_that_has_not_finished_offers_only_cancelling_it() {
    let menu = menu(true, true, false, false);
    assert_eq!(menu.actions, vec![Some(Action::CancelInstall)]);
    assert!(matches!(menu.items[0], MenuItem::Action { danger: true, .. }));
}

#[test]
fn every_action_row_has_an_action_and_every_separator_has_none() {
    let menu = menu(false, true, true, true);
    assert_eq!(menu.items.len(), menu.actions.len());
    for (row, action) in menu.items.iter().zip(&menu.actions) {
        assert_eq!(action.is_some(), !matches!(row, MenuItem::Separator));
    }
}
