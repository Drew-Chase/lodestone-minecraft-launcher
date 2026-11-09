use lodestone_platforms::{ContentType, ContentVersion};

use super::*;

/// A release with `id`, built for the Minecraft versions `minecraft`.
fn release(id: &str, minecraft: &[&str]) -> ContentVersion {
    ContentVersion {
        id: id.to_string(),
        project_id: None,
        name: None,
        version_number: format!("{id}-number"),
        content_type: ContentType::Modpack,
        minecraft_versions: minecraft.iter().map(|version| version.to_string()).collect(),
        loaders: vec!["fabric".to_string()],
        downloads: 0,
        date: None,
        changelog_url: None,
        files: Vec::new(),
        changelog: None,
        channel: None,
        dependencies: Vec::new(),
    }
}

/// The releases of a platform, newest first: `new`, the installed `cur`, then `old`.
fn listed() -> Vec<ContentVersion> {
    vec![release("new", &["1.21"]), release("cur", &["1.20"]), release("old", &["1.20"])]
}

#[test]
fn the_installed_release_stands_as_installed() {
    assert_eq!(standing(&listed(), Some("cur"), "cur"), Standing::Installed);
}

#[test]
fn a_release_listed_before_the_installed_one_is_newer_and_one_listed_after_it_is_older() {
    assert_eq!(standing(&listed(), Some("cur"), "new"), Standing::Newer);
    assert_eq!(standing(&listed(), Some("cur"), "old"), Standing::Older);
}

#[test]
fn with_the_installed_release_unknown_every_release_is_unknown() {
    assert_eq!(standing(&listed(), None, "new"), Standing::Unknown);
    assert_eq!(standing(&listed(), None, "cur"), Standing::Unknown);
}

#[test]
fn an_installed_release_the_platform_no_longer_lists_leaves_the_others_other() {
    assert_eq!(standing(&listed(), Some("gone"), "new"), Standing::Other);
}

#[test]
fn a_release_the_platform_does_not_list_is_other() {
    assert_eq!(standing(&listed(), Some("cur"), "absent"), Standing::Other);
}

#[test]
fn a_listing_from_an_earlier_open_is_dropped_and_the_current_one_is_kept() {
    let mut state = State {
        generation: 2,
        listing: Listing::Loading,
        ..State::default()
    };
    let _ = state.update(Message::Loaded {
        generation: 1,
        result: Ok(listed()),
    });
    assert_eq!(state.listing, Listing::Loading);
    let _ = state.update(Message::Loaded {
        generation: 2,
        result: Ok(listed()),
    });
    assert_eq!(state.listing, Listing::Loaded(listed()));
}

#[test]
fn a_release_is_found_in_the_listing_by_its_id() {
    let state = State {
        listing: Listing::Loaded(listed()),
        ..State::default()
    };
    assert_eq!(state.release("cur").map(|version| version.version_number.as_str()), Some("cur-number"));
    assert!(state.release("absent").is_none());
}
