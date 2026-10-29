use chrono::Local;

use super::*;

fn library_with(instances: Vec<Instance>) -> Library {
    let mut library = Library::default();
    library.set_instances(instances);
    library
}

#[test]
fn filters_narrow_the_list() {
    let mut library = library_with(Instance::seed_pack(Local::now()));
    assert_eq!(library.visible().len(), 48, "Recent shows everything");

    library.update(Message::FilterChanged(LibraryFilter::Pinned));
    assert!(library.visible().iter().all(|instance| instance.pinned), "Pinned shows only pinned");
    assert!(library.visible().len() < 48);
}

#[test]
fn modded_and_vanilla_tabs_are_disjoint() {
    let mut library = library_with(Instance::seed_pack(Local::now()));
    library.update(Message::FilterChanged(LibraryFilter::Modded));
    let modded = library.visible().clone();
    assert!(!modded.is_empty());
    assert!(modded.iter().all(|instance| instance.loader != Loader::Vanilla));

    library.update(Message::FilterChanged(LibraryFilter::Vanilla));
    let vanilla = library.visible().clone();
    assert!(!vanilla.is_empty());
    assert!(vanilla.iter().all(|instance| instance.loader == Loader::Vanilla));
}

#[test]
fn search_matches_by_name_case_insensitively() {
    let mut library = library_with(Instance::seed_pack(Local::now()));
    library.update(Message::SearchChanged("AETHER".to_string()));
    let visible = library.visible();
    assert!(!visible.is_empty(), "the first seed instance is 'Aether Legacy'");
    assert!(visible.iter().all(|instance| instance.name.to_lowercase().contains("aether")));

    library.update(Message::SearchChanged("no instance has this name".to_string()));
    assert!(library.visible().is_empty());
}

#[test]
fn visible_sorts_most_recent_first() {
    let library = library_with(Instance::seed_pack(Local::now()));
    let visible = library.visible();
    for pair in visible.windows(2) {
        assert!(pair[0].recency() >= pair[1].recency(), "instances must sort most-recent first");
    }
}

#[test]
fn scroll_belongs_to_the_page_not_the_filters() {
    let mut library = library_with(Instance::seed_pack(Local::now()));
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
