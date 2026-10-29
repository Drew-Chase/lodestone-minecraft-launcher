use chrono::{Duration, Local};

use super::instance::fixture;
use super::*;

/// Three instances: a pinned Fabric pack played just now, a vanilla world
/// installed three days ago, and a Forge pack installed just now.
fn sample() -> Vec<Instance> {
    let mut aether = fixture("aether", "Aether Legacy", Loader::Fabric);
    aether.pinned = true;
    aether.last_played = Some(Local::now());
    let mut vanilla = fixture("vanilla", "Vanilla Survival", Loader::Vanilla);
    vanilla.installed_at = Local::now() - Duration::days(3);
    let forge = fixture("sky", "Sky Factory 5", Loader::Forge);
    vec![aether, vanilla, forge]
}

fn library_with(instances: Vec<Instance>) -> Library {
    let mut library = Library::default();
    library.set_instances(instances);
    library
}

/// The names of the visible instances, in list order.
fn names(library: &Library) -> Vec<String> {
    library.visible().iter().map(|instance| instance.name.clone()).collect()
}

#[test]
fn filters_narrow_the_list() {
    let mut library = library_with(sample());
    assert_eq!(library.visible().len(), 3, "Recent shows everything");

    library.update(Message::FilterChanged(LibraryFilter::Pinned));
    assert_eq!(names(&library), ["Aether Legacy"], "Pinned shows only pinned");
}

#[test]
fn modded_and_vanilla_tabs_are_disjoint() {
    let mut library = library_with(sample());
    library.update(Message::FilterChanged(LibraryFilter::Modded));
    let mut modded = names(&library);
    modded.sort();
    assert_eq!(modded, ["Aether Legacy", "Sky Factory 5"]);

    library.update(Message::FilterChanged(LibraryFilter::Vanilla));
    assert_eq!(names(&library), ["Vanilla Survival"]);
}

#[test]
fn search_matches_by_name_case_insensitively() {
    let mut library = library_with(sample());
    library.update(Message::SearchChanged("AETHER".to_string()));
    assert_eq!(names(&library), ["Aether Legacy"]);

    library.update(Message::SearchChanged("no instance has this name".to_string()));
    assert!(library.visible().is_empty());
}

#[test]
fn visible_sorts_most_recent_first() {
    let library = library_with(sample());
    let visible = library.visible();
    for pair in visible.windows(2) {
        assert!(pair[0].recency() >= pair[1].recency(), "instances must sort most-recent first");
    }
}

#[test]
fn the_banner_is_absent_without_instances() {
    assert!(Library::default().hero().is_none(), "an empty library reserves no room for the banner");
    assert!(library_with(sample()).hero().is_some(), "one instance brings the banner back");
}

#[test]
fn scroll_belongs_to_the_page_not_the_filters() {
    let mut library = library_with(sample());
    library.update(Message::Scrolled(2500.0));
    assert_eq!(library.scroll_y, 2500.0);

    // Filtering, searching and switching views must not jump the page.
    library.update(Message::FilterChanged(LibraryFilter::Modded));
    library.update(Message::SearchChanged("a".to_string()));
    library.update(Message::ViewChanged(LibraryView::Table));
    assert_eq!(library.scroll_y, 2500.0, "the page scroll is the user's position");
    assert_eq!(library.view, LibraryView::Table);
}

#[test]
fn grid_columns_follow_width_with_a_cap() {
    assert_eq!(LibraryView::Grid.columns(340.0), 1);
    assert_eq!(LibraryView::Grid.columns(1156.0), 3, "1280px window minus rail and padding");
    assert_eq!(LibraryView::Grid.columns(4000.0), 4, "wide windows cap at four columns");
    assert_eq!(LibraryView::Compact.columns(4000.0), 1);
    assert_eq!(LibraryView::Table.columns(4000.0), 1);
}

#[test]
fn row_pitches_match_the_fixed_component_heights() {
    assert_eq!(LibraryView::Grid.row_pitch(), CARD_HEIGHT + 24.0);
    assert_eq!(LibraryView::Compact.row_pitch(), COMPACT_ROW_HEIGHT + 24.0);
    assert_eq!(LibraryView::Table.row_pitch(), TABLE_ROW_HEIGHT);
}
