use super::*;
use lodestone_platforms::ContentType;

/// A search hit for the project `id`.
fn hit(id: &str) -> SearchResult {
    SearchResult {
        id: id.to_string(),
        slug: None,
        name: format!("Mod {id}"),
        author: None,
        description: String::new(),
        downloads: 0,
        icon_url: None,
        banner_url: None,
        web_url: None,
        updated: None,
        content_type: ContentType::Mod,
    }
}

/// A release with the given id.
fn release(id: &str) -> ContentVersion {
    ContentVersion {
        id: id.to_string(),
        project_id: Some("sodium".to_string()),
        name: None,
        version_number: id.to_string(),
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
    }
}

/// A search for `query` on Modrinth, for the jar `sodium.jar`.
fn associate(query: &str) -> Associate {
    Associate::new("sodium.jar".to_string(), query.to_string(), Source::Modrinth)
}

#[test]
fn a_query_changes_the_search_text() {
    let mut dialog = Dialog::Associate(Box::new(associate("sodium")));
    dialog.local(&Message::Query("lithium".to_string()));
    let Dialog::Associate(associate) = &dialog else {
        panic!("the dialog is still the search");
    };
    assert_eq!(associate.query, "lithium");
}

#[test]
fn choosing_a_release_marks_it_and_going_back_drops_the_pick() {
    let mut search = associate("sodium");
    search.picked = Some(Picked {
        project: hit("sodium"),
        releases: Load::Loaded(vec![release("a"), release("b")]),
        chosen: None,
        jar_sha1: None,
    });
    let mut dialog = Dialog::Associate(Box::new(search));

    dialog.local(&Message::Choose(1));
    let Dialog::Associate(search) = &dialog else {
        panic!("the dialog is still the search");
    };
    assert_eq!(search.chosen_release().map(|release| release.id.as_str()), Some("b"));

    dialog.local(&Message::Back);
    let Dialog::Associate(search) = &dialog else {
        panic!("the dialog is still the search");
    };
    assert!(search.picked.is_none(), "back returns to the results");
}

#[test]
fn no_release_is_chosen_until_the_releases_load() {
    let mut search = associate("sodium");
    search.picked = Some(Picked {
        project: hit("sodium"),
        releases: Load::Loading,
        chosen: Some(0),
        jar_sha1: None,
    });
    assert!(search.chosen_release().is_none());
}

#[test]
fn only_the_tab_answers_the_local_messages() {
    assert!(Message::Close.is_local() && Message::Query(String::new()).is_local() && Message::Choose(0).is_local() && Message::Back.is_local());
    assert!(
        !Message::Search.is_local()
            && !Message::Platform(Source::CurseForge).is_local()
            && !Message::Pick(0).is_local()
            && !Message::Confirm.is_local()
            && !Message::Use(0).is_local()
            && !Message::Open(0).is_local()
    );
}

#[test]
fn a_version_drops_the_text_the_jar_name_shares_with_it() {
    assert_eq!(version_label("appleskin-forge-mc1.18.2-2.4.0", "appleskin-forge-mc1.18.2-2.4.1"), "2.4.0");
}

#[test]
fn a_version_that_shares_nothing_keeps_all_of_it() {
    assert_eq!(version_label("2.4.1", "appleskin-forge-mc1.18.2-2.4.1"), "2.4.1");
}

#[test]
fn a_shared_version_with_no_digit_left_keeps_all_of_it() {
    assert_eq!(version_label("appleskin-forge", "appleskin-forge"), "appleskin-forge");
}

#[test]
fn the_jar_stem_drops_its_extension() {
    assert_eq!(jar_stem("sodium-1.0.jar"), "sodium-1.0");
    assert_eq!(jar_stem("sodium-1.0.jar.disabled"), "sodium-1.0");
}

#[test]
fn only_the_kinds_a_mod_uses_a_library_for_are_dependents() {
    assert_eq!(DependentKind::of(ModDependencyKind::Required), Some(DependentKind::Required));
    assert_eq!(DependentKind::of(ModDependencyKind::Optional), Some(DependentKind::Optional));
    assert_eq!(DependentKind::of(ModDependencyKind::Recommended), Some(DependentKind::Recommended));
    assert_eq!(DependentKind::of(ModDependencyKind::Suggested), Some(DependentKind::Suggested));
    for kind in [
        ModDependencyKind::LoadBefore,
        ModDependencyKind::Incompatible,
        ModDependencyKind::Discouraged,
    ] {
        assert_eq!(DependentKind::of(kind), None, "{kind:?} does not make a mod a dependent");
    }
}
