use super::super::Source;
use super::super::query::{Key, PAGE_SIZE};
use super::*;
use lodestone_platforms::{Facets, SearchSort};

/// A page on the modpacks tab, with the platform's `facets` loaded for it.
fn page_with(facets: Facets) -> Discover {
    let mut page = Discover::new(Source::Modrinth);
    page.facets.insert((Source::Modrinth, ContentType::Modpack), facets);
    page
}

/// Whether the effects ask the platform for a search.
fn searches(effects: &[Effect]) -> bool {
    effects.iter().any(|effect| matches!(effect, Effect::Search(_)))
}

#[test]
fn a_pick_is_added_and_picking_it_again_drops_it() {
    let mut page = page_with(Facets {
        multiple: true,
        ..Facets::default()
    });
    let effects = page.filter_changed(FilterMessage::Pick(Kind::Category, "magic".to_string()));
    assert_eq!(page.filters.categories, ["magic"]);
    assert!(searches(&effects), "the filtered search starts");
    page.filter_changed(FilterMessage::Pick(Kind::Category, "magic".to_string()));
    assert!(page.filters.categories.is_empty());
}

#[test]
fn a_platform_that_takes_one_value_keeps_only_the_last_pick() {
    let mut page = page_with(Facets {
        multiple: false,
        ..Facets::default()
    });
    page.filter_changed(FilterMessage::Pick(Kind::Loader, "fabric".to_string()));
    page.filter_changed(FilterMessage::Pick(Kind::Loader, "forge".to_string()));
    assert_eq!(page.filters.loaders, ["forge"]);
}

#[test]
fn a_platform_that_takes_many_values_keeps_every_pick() {
    let mut page = page_with(Facets {
        multiple: true,
        ..Facets::default()
    });
    page.filter_changed(FilterMessage::Pick(Kind::Loader, "fabric".to_string()));
    page.filter_changed(FilterMessage::Pick(Kind::Loader, "forge".to_string()));
    assert_eq!(page.filters.loaders, ["fabric", "forge"]);
}

#[test]
fn the_same_picks_in_another_order_make_the_same_search_key() {
    let facets = || Facets {
        multiple: true,
        ..Facets::default()
    };
    let mut first = page_with(facets());
    first.filter_changed(FilterMessage::Pick(Kind::Category, "magic".to_string()));
    first.filter_changed(FilterMessage::Pick(Kind::Category, "adventure".to_string()));
    assert_eq!(first.filters.categories, ["adventure", "magic"]);

    let mut second = page_with(facets());
    second.filter_changed(FilterMessage::Pick(Kind::Category, "adventure".to_string()));
    second.filter_changed(FilterMessage::Pick(Kind::Category, "magic".to_string()));
    assert_eq!(first.key(), second.key());
}

#[test]
fn environments_are_listed_client_first_whatever_the_order_of_the_picks() {
    let mut page = page_with(Facets {
        environments: true,
        multiple: true,
        ..Facets::default()
    });
    page.filter_changed(FilterMessage::Environment(Side::Server));
    page.filter_changed(FilterMessage::Environment(Side::Client));
    assert_eq!(page.filters.environments, [Side::Client, Side::Server]);
    page.filter_changed(FilterMessage::Environment(Side::Client));
    assert_eq!(page.filters.environments, [Side::Server]);
}

#[test]
fn an_exclusion_is_its_own_list_and_does_not_pick_anything() {
    let mut page = page_with(Facets {
        exclusions: true,
        multiple: true,
        ..Facets::default()
    });
    page.filter_changed(FilterMessage::Exclude(Kind::Category, "optimization".to_string()));
    page.filter_changed(FilterMessage::Exclude(Kind::Loader, "forge".to_string()));
    assert_eq!(page.filters.excluded_categories, ["optimization"]);
    assert_eq!(page.filters.excluded_loaders, ["forge"]);
    assert!(page.filters.categories.is_empty());
}

#[test]
fn the_panel_changes_ask_for_no_search() {
    let mut page = page_with(Facets {
        snapshot_versions: vec!["26.3-snapshot-1".to_string()],
        multiple: true,
        ..Facets::default()
    });
    assert!(page.filter_changed(FilterMessage::ShowAllVersions(true)).is_empty());
    assert!(page.panel.show_all_versions);
    assert!(page.filter_changed(FilterMessage::VersionSearch("26.3".to_string())).is_empty());
    assert_eq!(page.panel.version_search, "26.3");
    let expand = FilterMessage::Expand {
        section: "versions".to_string(),
        expanded: true,
    };
    assert!(page.filter_changed(expand).is_empty());
    assert!(page.panel.expanded.contains("versions"));
}

#[test]
fn clearing_drops_every_pick() {
    let mut page = page_with(Facets {
        open_source: true,
        multiple: true,
        ..Facets::default()
    });
    page.filter_changed(FilterMessage::Pick(Kind::Category, "magic".to_string()));
    page.filter_changed(FilterMessage::OpenSource(true));
    assert!(!page.filters.is_empty());
    page.filter_changed(FilterMessage::Clear);
    assert!(page.filters.is_empty());
}

#[test]
fn the_picks_reach_the_query_the_search_asks_for() {
    let key = Key {
        source: Source::Modrinth,
        content: ContentType::Mod,
        text: "sodium".to_string(),
        sort: SearchSort::Relevance,
        filters: Filters {
            categories: vec!["magic".to_string()],
            loaders: vec!["fabric".to_string(), "quilt".to_string()],
            game_versions: vec!["1.20.1".to_string()],
            environments: vec![Side::Client],
            open_source: true,
            excluded_categories: vec!["optimization".to_string()],
            excluded_loaders: vec!["forge".to_string()],
        },
    };
    let query = key.query(20);
    assert_eq!(query.text, "sodium");
    assert_eq!((query.offset, query.limit), (20, PAGE_SIZE));
    assert_eq!(query.categories, ["magic"]);
    assert_eq!(query.loaders, ["fabric", "quilt"]);
    assert_eq!(query.game_versions, ["1.20.1"]);
    assert_eq!(query.environments, [Side::Client]);
    assert!(query.open_source);
    assert_eq!(query.excluded_categories, ["optimization"]);
    assert_eq!(query.excluded_loaders, ["forge"]);
}

#[test]
fn the_modpacks_tab_takes_no_version_from_its_instance() {
    assert!(!Discover::new(Source::Modrinth).takes_target_version());
    let mut mods = Discover::new(Source::Modrinth);
    mods.content = ContentType::Mod;
    assert!(mods.takes_target_version());
}
