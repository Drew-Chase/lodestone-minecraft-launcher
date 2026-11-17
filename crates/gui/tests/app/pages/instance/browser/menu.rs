use super::*;

const FILE: Target = Target { is_folder: false, count: 1 };
const FOLDER: Target = Target { is_folder: true, count: 1 };

fn actions(menu: &Menu) -> Vec<Action> {
    menu.actions.iter().flatten().copied().collect()
}

#[test]
fn every_item_has_its_action_in_the_same_place() {
    for menu in [
        entry(FILE, false, true),
        entry(FOLDER, false, true),
        folder(false, true),
        entry(FILE, true, false),
    ] {
        assert_eq!(menu.items.len(), menu.actions.len());
        for (item, action) in menu.items.iter().zip(&menu.actions) {
            assert_eq!(matches!(item, MenuItem::Separator), action.is_none());
        }
        assert!(!matches!(menu.items.first(), Some(MenuItem::Separator)));
        assert!(!matches!(menu.items.last(), Some(MenuItem::Separator)));
    }
}

#[test]
fn a_file_opens_two_ways_and_a_folder_takes_a_paste() {
    assert_eq!(
        actions(&entry(FILE, false, true)),
        [
            Action::Open,
            Action::OpenExternal,
            Action::Reveal,
            Action::Copy,
            Action::Cut,
            Action::CopyPath,
            Action::Rename,
            Action::Delete,
        ]
    );
    let folder = actions(&entry(FOLDER, false, true));
    assert!(folder.contains(&Action::Paste));
    assert!(!folder.contains(&Action::OpenExternal));
    assert!(
        !actions(&entry(FOLDER, false, false)).contains(&Action::Paste),
        "nothing to paste, no Paste row"
    );
}

#[test]
fn several_entries_offer_only_what_applies_to_all_of_them() {
    let several = Target { is_folder: false, count: 3 };
    assert_eq!(actions(&entry(several, false, true)), [Action::Copy, Action::Cut, Action::Delete]);
}

#[test]
fn a_read_only_tab_offers_nothing_that_changes_the_folder() {
    for menu in [
        entry(FILE, true, true),
        entry(FOLDER, true, true),
        folder(true, true),
        entry(Target { is_folder: false, count: 4 }, true, true),
    ] {
        assert!(actions(&menu).iter().all(|action| !action.changes()));
    }
    assert_eq!(
        actions(&entry(FILE, true, true)),
        [Action::Open, Action::OpenExternal, Action::Reveal, Action::Copy, Action::CopyPath]
    );
    assert_eq!(actions(&folder(true, true)), [Action::Reveal, Action::CopyPath]);
}

#[test]
fn the_folder_menu_makes_new_entries_and_pastes_when_it_can() {
    assert_eq!(
        actions(&folder(false, true)),
        [Action::Paste, Action::NewFolder, Action::NewFile, Action::Reveal, Action::CopyPath]
    );
    assert_eq!(
        actions(&folder(false, false)),
        [Action::NewFolder, Action::NewFile, Action::Reveal, Action::CopyPath]
    );
}

#[test]
fn only_delete_is_drawn_as_dangerous() {
    let menu = entry(FILE, false, true);
    for (item, action) in menu.items.iter().zip(&menu.actions) {
        if let MenuItem::Action { danger, .. } = item {
            assert_eq!(*danger, *action == Some(Action::Delete));
        }
    }
}
