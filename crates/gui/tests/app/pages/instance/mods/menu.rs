use super::*;
use crate::app::pages::instance::files::ModFile;
use crate::app::shared_library::ModSource;
use lodestone_platforms::{ContentType, ContentVersion};

/// A mod row, with or without a recorded source and an update.
fn row(update: bool, source: bool) -> Row {
    Row {
        file: ModFile {
            file_name: "sodium.jar".to_string(),
            enabled: true,
            name: "Sodium".to_string(),
            author: None,
            version: None,
            id: None,
            dependencies: Vec::new(),
            icon: None,
        },
        source: source.then(|| ModSource {
            instance: "a".to_string(),
            platform: "modrinth".to_string(),
            project_id: "sodium".to_string(),
            version_id: "sodium-1".to_string(),
            name: "Sodium".to_string(),
            file_name: "sodium.jar".to_string(),
            sha1: None,
        }),
        update: update.then(|| ContentVersion {
            id: "sodium-2".to_string(),
            project_id: Some("sodium".to_string()),
            name: None,
            version_number: "0.6".to_string(),
            content_type: ContentType::Mod,
            minecraft_versions: Vec::new(),
            loaders: Vec::new(),
            downloads: 0,
            date: None,
            changelog_url: None,
            files: Vec::new(),
            changelog: None,
            channel: None,
            dependencies: Vec::new(),
        }),
        library: false,
    }
}

/// A library: a mod that other mods in the folder name as a dependency.
fn library(source: bool) -> Row {
    Row {
        library: true,
        ..row(false, source)
    }
}

#[test]
fn an_unassociated_mod_offers_only_associating_and_deleting() {
    assert_eq!(
        menu(&row(false, false), false, false).actions,
        [Some(Action::Associate), Some(Action::Reveal), None, Some(Action::Delete)]
    );
}

#[test]
fn an_associated_mod_offers_switching_and_its_dependencies() {
    assert_eq!(
        menu(&row(false, true), false, false).actions,
        [
            Some(Action::Switch),
            Some(Action::Dependencies),
            None,
            Some(Action::Associate),
            Some(Action::Reveal),
            None,
            Some(Action::Delete)
        ]
    );
}

#[test]
fn an_update_leads_the_menu_when_the_platform_has_one() {
    assert_eq!(
        menu(&row(true, true), false, false).actions,
        [
            Some(Action::Update),
            Some(Action::Switch),
            Some(Action::Dependencies),
            None,
            Some(Action::Associate),
            Some(Action::Reveal),
            None,
            Some(Action::Delete),
        ]
    );
}

#[test]
fn an_update_row_names_the_release_it_installs() {
    let menu = menu(&row(true, true), false, false);
    let MenuItem::Action { label, .. } = &menu.items[0] else {
        panic!("the first row is an action");
    };
    assert!(label.contains("0.6"), "got {label}");
}

#[test]
fn a_mod_with_an_action_in_flight_has_no_menu() {
    let menu = menu(&row(true, true), true, false);
    assert!(menu.items.is_empty() && menu.actions.is_empty());
}

#[test]
fn every_item_has_its_action_in_the_same_place() {
    let menu = menu(&row(true, true), false, false);
    assert_eq!(menu.items.len(), menu.actions.len());
    for (item, action) in menu.items.iter().zip(&menu.actions) {
        assert_eq!(matches!(item, MenuItem::Separator), action.is_none());
    }
}

#[test]
fn a_locked_mod_offers_no_update_switch_or_delete() {
    assert_eq!(
        menu(&row(true, true), false, true).actions,
        [Some(Action::Dependencies), None, Some(Action::Associate), Some(Action::Reveal)]
    );
}

#[test]
fn a_locked_unassociated_mod_offers_only_associating_and_revealing() {
    assert_eq!(
        menu(&row(false, false), false, true).actions,
        [Some(Action::Associate), Some(Action::Reveal)]
    );
}

#[test]
fn a_locked_menu_keeps_its_items_and_actions_in_step() {
    let menu = menu(&row(true, true), false, true);
    assert_eq!(menu.items.len(), menu.actions.len());
    for (item, action) in menu.items.iter().zip(&menu.actions) {
        assert_eq!(matches!(item, MenuItem::Separator), action.is_none());
    }
}

#[test]
fn a_library_offers_its_dependents_after_its_dependencies() {
    assert_eq!(
        menu(&library(true), false, false).actions,
        [
            Some(Action::Switch),
            Some(Action::Dependencies),
            Some(Action::Dependents),
            None,
            Some(Action::Associate),
            Some(Action::Reveal),
            None,
            Some(Action::Delete)
        ]
    );
}

#[test]
fn a_library_without_a_source_offers_its_dependents_alone() {
    assert_eq!(
        menu(&library(false), false, false).actions,
        [
            Some(Action::Dependents),
            None,
            Some(Action::Associate),
            Some(Action::Reveal),
            None,
            Some(Action::Delete)
        ]
    );
}

#[test]
fn a_locked_library_still_offers_its_dependents() {
    assert_eq!(
        menu(&library(true), false, true).actions,
        [
            Some(Action::Dependencies),
            Some(Action::Dependents),
            None,
            Some(Action::Associate),
            Some(Action::Reveal)
        ]
    );
}

#[test]
fn a_mod_nothing_names_offers_no_dependents() {
    assert!(!menu(&row(false, true), false, false).actions.contains(&Some(Action::Dependents)));
}
