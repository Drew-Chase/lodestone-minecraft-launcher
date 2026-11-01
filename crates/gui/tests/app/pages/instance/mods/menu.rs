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
    }
}

#[test]
fn an_unassociated_mod_offers_only_associating_and_deleting() {
    assert_eq!(
        menu(&row(false, false), false).actions,
        [Some(Action::Associate), None, Some(Action::Delete)]
    );
}

#[test]
fn an_associated_mod_offers_switching_and_its_dependencies() {
    assert_eq!(
        menu(&row(false, true), false).actions,
        [
            Some(Action::Switch),
            Some(Action::Dependencies),
            None,
            Some(Action::Associate),
            None,
            Some(Action::Delete)
        ]
    );
}

#[test]
fn an_update_leads_the_menu_when_the_platform_has_one() {
    assert_eq!(
        menu(&row(true, true), false).actions,
        [
            Some(Action::Update),
            Some(Action::Switch),
            Some(Action::Dependencies),
            None,
            Some(Action::Associate),
            None,
            Some(Action::Delete),
        ]
    );
}

#[test]
fn an_update_row_names_the_release_it_installs() {
    let menu = menu(&row(true, true), false);
    let MenuItem::Action { label, .. } = &menu.items[0] else {
        panic!("the first row is an action");
    };
    assert!(label.contains("0.6"), "got {label}");
}

#[test]
fn a_mod_with_an_action_in_flight_has_no_menu() {
    let menu = menu(&row(true, true), true);
    assert!(menu.items.is_empty() && menu.actions.is_empty());
}

#[test]
fn every_item_has_its_action_in_the_same_place() {
    let menu = menu(&row(true, true), false);
    assert_eq!(menu.items.len(), menu.actions.len());
    for (item, action) in menu.items.iter().zip(&menu.actions) {
        assert_eq!(matches!(item, MenuItem::Separator), action.is_none());
    }
}
