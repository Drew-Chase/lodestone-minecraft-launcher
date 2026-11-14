//! Tests for `window`: the rows a layout builds, the spacers that stand in for the rest, the shape of each layout, and where the
//! results sit in a page.

use super::*;
use crate::app::pages::discover::Source;
use lodestone_platforms::{ContentType, Facets, SearchResult};

/// The height a column of the built rows and spacers takes, with a gap between each two children.
fn stacked_height(rows: Rows, row_height: f32, pitch: f32, gap: f32) -> f32 {
    let mut heights: Vec<f32> = Vec::new();
    heights.extend(top_spacer(rows, pitch, gap));
    heights.extend(std::iter::repeat_n(row_height, rows.end - rows.first));
    heights.extend(bottom_spacer(rows, pitch, gap));
    heights.iter().sum::<f32>() + (heights.len().saturating_sub(1)) as f32 * gap
}

/// The height of the whole list, each row and the gap between rows.
fn full_height(total: usize, row_height: f32, gap: f32) -> f32 {
    if total == 0 {
        0.0
    } else {
        total as f32 * row_height + (total - 1) as f32 * gap
    }
}

/// A hit with `id`, which the page lists.
fn hit(id: &str) -> SearchResult {
    SearchResult {
        id: id.to_string(),
        slug: None,
        name: format!("Pack {id}"),
        author: None,
        description: String::new(),
        downloads: 0,
        icon_url: None,
        banner_url: None,
        web_url: None,
        updated: None,
        content_type: ContentType::Modpack,
    }
}

/// A browse of `hits` hits on Modrinth, with no filters loaded.
fn page_with(hits: usize) -> Discover {
    let mut page = Discover::new(Source::Modrinth);
    page.results.hits = (0..hits).map(|index| hit(&index.to_string())).collect();
    page
}

/// Loads the filters of a modpack browse on Modrinth, so the filter column has a section to show.
fn with_filters(page: &mut Discover) {
    page.facets.insert(
        (Source::Modrinth, ContentType::Modpack),
        Facets {
            game_versions: vec!["1.20.1".to_string()],
            ..Facets::default()
        },
    );
}

#[test]
fn rows_on_the_first_screen_build_the_screen_and_the_rows_below_it() {
    // Rows 100 px apart, a 300 px window, nothing scrolled: rows 0 to 2 are on screen, and two more are built below.
    assert_eq!(
        rows(0.0, 0.0, 300.0, 100.0, 100),
        Rows {
            first: 0,
            end: 5,
            total: 100
        }
    );
}

#[test]
fn rows_in_the_middle_of_the_list_build_one_above_and_two_below() {
    // Scrolled 1050 px: row 10 is the first on screen, so row 9 is built above it.
    assert_eq!(
        rows(1050.0, 0.0, 300.0, 100.0, 100),
        Rows {
            first: 9,
            end: 16,
            total: 100
        }
    );
}

#[test]
fn rows_at_the_end_of_the_list_stop_at_its_last_row() {
    assert_eq!(
        rows(1100.0, 0.0, 300.0, 100.0, 12),
        Rows {
            first: 10,
            end: 12,
            total: 12
        }
    );
}

#[test]
fn rows_past_the_end_of_the_list_still_build_its_last_row() {
    assert_eq!(rows(10_000.0, 0.0, 300.0, 100.0, 5), Rows { first: 4, end: 5, total: 5 });
}

#[test]
fn the_results_start_below_the_content_above_them() {
    // The content scrolled 50 px, but the results start 200 px down: nothing of them is above the window yet.
    assert_eq!(
        rows(50.0, 200.0, 300.0, 100.0, 100),
        Rows {
            first: 0,
            end: 5,
            total: 100
        }
    );
}

#[test]
fn an_empty_list_builds_no_rows() {
    assert_eq!(rows(0.0, 0.0, 800.0, 100.0, 0), Rows { first: 0, end: 0, total: 0 });
}

#[test]
fn spacers_stand_in_for_the_rows_not_built() {
    let rows = Rows {
        first: 9,
        end: 16,
        total: 100,
    };
    assert_eq!(top_spacer(rows, 100.0, 0.0), Some(900.0));
    assert_eq!(bottom_spacer(rows, 100.0, 0.0), Some(8400.0));
}

#[test]
fn a_spacer_leaves_out_the_gap_the_line_puts_under_it() {
    // A line of 330 px rows with a 16 px gap: the rows before row 9 take nine pitches (346 px each), less the gap under the spacer.
    let rows = Rows {
        first: 9,
        end: 16,
        total: 100,
    };
    assert_eq!(top_spacer(rows, 346.0, 16.0), Some(9.0 * 346.0 - 16.0));
    assert_eq!(bottom_spacer(rows, 346.0, 16.0), Some(84.0 * 346.0 - 16.0));
}

#[test]
fn there_is_no_spacer_where_no_row_lies() {
    let top = Rows {
        first: 0,
        end: 5,
        total: 100,
    };
    assert_eq!(top_spacer(top, 100.0, 0.0), None);
    let bottom = Rows {
        first: 10,
        end: 12,
        total: 12,
    };
    assert_eq!(bottom_spacer(bottom, 100.0, 0.0), None);
}

#[test]
fn built_rows_and_spacers_take_the_height_of_the_whole_list() {
    // The grid's rows are 330 px with a 16 px gap; the compact rows 172 px with the same gap; the table's 88 px with none.
    for (total, first, end) in [(50, 9, 16), (50, 0, 5), (50, 45, 50), (7, 2, 3), (1, 0, 1)] {
        let rows = Rows { first, end, total };
        for (row_height, gap) in [(330.0, 16.0), (172.0, 16.0), (88.0, 0.0)] {
            let pitch = row_height + gap;
            assert!(
                (stacked_height(rows, row_height, pitch, gap) - full_height(total, row_height, gap)).abs() < 0.001,
                "{total} rows, built {first}..{end}, {row_height} px rows with a {gap} px gap"
            );
        }
    }
}

#[test]
fn grid_columns_fit_at_the_minimum_width_up_to_four() {
    assert_eq!(grid_columns(100.0), 1, "narrower than one card still builds one column");
    assert_eq!(grid_columns(300.0), 1);
    assert_eq!(grid_columns(592.0), 2);
    assert_eq!(grid_columns(888.0), 3);
    assert_eq!(grid_columns(1184.0), 4);
    assert_eq!(grid_columns(2000.0), 4, "never more than four");
}

#[test]
fn compact_rows_sit_two_to_a_line_from_760_px() {
    assert_eq!(compact_columns(320.0), 1);
    assert_eq!(compact_columns(759.0), 1);
    assert_eq!(compact_columns(760.0), 2);
}

#[test]
fn each_layout_has_its_row_height_gap_and_columns() {
    let grid = shape(DiscoverView::Grid, 900.0);
    assert_eq!((grid.columns, grid.row_height, grid.gap), (3, 330.0, 16.0));
    assert_eq!(grid.pitch(), 346.0);

    let compact = shape(DiscoverView::Compact, 900.0);
    assert_eq!((compact.columns, compact.row_height, compact.gap), (2, 172.0, 16.0));
    assert_eq!(compact.pitch(), 188.0);

    let table = shape(DiscoverView::Table, 900.0);
    assert_eq!((table.columns, table.row_height, table.gap), (1, 88.0, 0.0));
    assert_eq!(table.pitch(), 88.0);
}

#[test]
fn the_results_are_as_wide_as_the_window_less_the_rail_the_margins_and_the_filter_column() {
    let rail = crate::app::components::sidebar::WIDTH;
    let mut page = page_with(0);
    assert_eq!(
        results_width(&page, 1400.0),
        1400.0 - rail - SIDE - SIDE,
        "with no filters loaded, a margin stands in for the filter column"
    );

    with_filters(&mut page);
    assert_eq!(
        results_width(&page, 1400.0),
        1400.0 - rail - (sidebar::WIDTH + scrollbar::TRACK_WIDTH) - SIDE,
        "with filters, the filter column takes its width"
    );
}

#[test]
fn the_results_start_below_the_hero_and_the_toolbar_of_a_browse() {
    let empty = page_with(0);
    let without_hero = PAGE_GAP + toolbar::HEIGHT + PAGE_GAP;
    assert_eq!(results_top(&empty, 1400.0), without_hero);

    let browsing = page_with(3);
    assert_eq!(
        results_top(&browsing, 1400.0),
        PAGE_GAP + hero::HEIGHT + PAGE_GAP + toolbar::HEIGHT + PAGE_GAP,
        "a browse's hero sits above the filters"
    );

    let mut searching = page_with(3);
    searching.text = "sodium".to_string();
    assert_eq!(results_top(&searching, 1400.0), without_hero, "a search shows no hero");
}

#[test]
fn the_table_header_sits_above_its_rows() {
    let plain = results_top(&page_with(3), 1400.0);
    let mut table = page_with(3);
    table.view = DiscoverView::Table;
    assert_eq!(results_top(&table, 1400.0), plain + TABLE_HEADER_HEIGHT);
}

#[test]
fn the_narrow_filter_panel_adds_its_height_only_while_it_is_open() {
    let mut page = page_with(3);
    with_filters(&mut page);
    let closed = results_top(&page, 800.0);

    page.filters_open = true;
    assert_eq!(results_top(&page, 800.0), closed + FILTERS_PANEL_HEIGHT + PAGE_GAP);
    assert_eq!(results_top(&page, 1400.0), closed, "the wide window keeps its filters beside the results");
}
