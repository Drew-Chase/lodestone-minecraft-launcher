//! Tests for `pictures`: which pictures the screen asks for, which it keeps, and which the sweep lets go. The sweep takes the
//! time in, so the 30-second rule is tested without waiting. The icon size depends on the window's scale, which is one global,
//! so these tests check which picture is asked for and not its exact pixel size.

use std::time::{Duration, Instant};

use iced::Size;
use iced::widget::image::Handle;
use lodestone_platforms::{ContentType, SearchResult};

use super::*;
use crate::app::pages::discover::Source;
use crate::app::pages::discover::types::Artwork;
use crate::app::retention::IDLE;

/// A hit with `id`, whose banner and icon are the platform's pictures of that id.
fn hit(id: &str) -> SearchResult {
    SearchResult {
        id: id.to_string(),
        slug: None,
        name: format!("Pack {id}"),
        author: None,
        description: String::new(),
        downloads: 0,
        icon_url: Some(icon_url(id)),
        banner_url: Some(banner_url(id)),
        web_url: None,
        updated: None,
        content_type: ContentType::Modpack,
    }
}

fn banner_url(id: &str) -> String {
    format!("https://cdn.example/{id}/banner.png")
}

fn icon_url(id: &str) -> String {
    format!("https://cdn.example/{id}/icon.png")
}

/// A browse of `hits` hits on Modrinth, in the grid.
fn page_with(hits: usize) -> Discover {
    let mut page = Discover::new(Source::Modrinth);
    page.results.hits = (0..hits).map(|index| hit(&index.to_string())).collect();
    page
}

/// The window the pages are laid out in: 1400 px by 900 px.
fn screen() -> Size {
    Size::new(1400.0, 900.0)
}

/// A picture standing in for a loaded one.
fn picture() -> Handle {
    Handle::from_rgba(1, 1, vec![0, 0, 0, 255])
}

#[test]
fn the_screen_starts_as_many_pictures_as_the_loads_have_room_for_nearest_the_top_first() {
    let mut page = page_with(40);
    let asked = page.wanted(screen());

    assert_eq!(asked.len(), ICONS_AT_ONCE);
    assert_eq!(
        asked[0],
        Asset::Banner(banner_url("0")),
        "the hero's banner, the first hit's, comes first"
    );
    assert!(page.requested.contains(&Asset::Banner(banner_url("0"))));
}

#[test]
fn a_picture_below_the_screen_is_not_asked_for() {
    let mut page = page_with(40);
    page.wanted(screen());

    assert!(
        !page.requested.contains(&Asset::Banner(banner_url("30"))),
        "hit 30 is far below the first screen"
    );
}

#[test]
fn the_grid_asks_for_each_hits_banner_and_title_icon_and_the_list_for_icons_only() {
    let grid = page_with(3);
    let assets = grid.on_screen(screen());
    assert!(assets.contains(&Asset::Banner(banner_url("1"))));
    assert!(
        assets
            .iter()
            .any(|asset| matches!(asset, Asset::Icon { url, .. } if url == &icon_url("1")))
    );

    let mut table = page_with(3);
    table.view = DiscoverView::Table;
    let assets = table.on_screen(screen());
    assert!(
        !assets.iter().any(|asset| matches!(asset, Asset::Banner(url) if url == &banner_url("1"))),
        "table rows draw no banner"
    );
    assert!(
        assets
            .iter()
            .any(|asset| matches!(asset, Asset::Icon { url, .. } if url == &icon_url("1")))
    );
}

#[test]
fn a_picture_that_leaves_the_screen_is_forgotten_and_asked_for_again_when_it_returns() {
    let mut page = page_with(40);
    page.wanted(screen());
    page.picture_ready(Asset::Banner(banner_url("0")), Some(picture()));

    // The screen moves far away while nothing uses the picture, and the sweep drops it.
    page.scroll_y = 5000.0;
    let later = Instant::now() + IDLE + Duration::from_secs(1);
    assert_eq!(page.sweep_pictures(later, screen(), true), 1);
    assert!(
        !page.requested.contains(&Asset::Banner(banner_url("0"))),
        "the page forgets the picture it dropped"
    );

    // Back on the first screen, the picture is queued again.
    page.scroll_y = 0.0;
    page.wanted(screen());
    assert!(page.requested.contains(&Asset::Banner(banner_url("0"))));
}

#[test]
fn a_picture_on_the_screen_stays_while_the_page_is_on_show_and_expires_once_it_is_left() {
    let mut page = page_with(40);
    page.wanted(screen());
    page.picture_ready(Asset::Banner(banner_url("0")), Some(picture()));
    let later = Instant::now() + IDLE + Duration::from_secs(10);

    assert_eq!(page.sweep_pictures(later, screen(), true), 0, "on the screen, the picture is in use");
    assert_eq!(
        page.sweep_pictures(later + IDLE + Duration::from_secs(1), screen(), false),
        1,
        "and it expires once the page is left"
    );
}

#[test]
fn a_picture_off_the_screen_expires_even_while_the_page_is_on_show() {
    let mut page = page_with(40);
    page.picture_ready(Asset::Banner(banner_url("35")), Some(picture()));
    let later = Instant::now() + IDLE + Duration::from_secs(1);

    assert_eq!(page.sweep_pictures(later, screen(), true), 1);
}

#[test]
fn a_picture_that_could_not_be_had_stands_the_scene_in() {
    let mut page = page_with(3);
    page.picture_ready(Asset::Banner(banner_url("1")), None);

    assert!(matches!(page.artwork(Some(banner_url("1").as_str())), Artwork::Scene));
    assert!(
        matches!(page.artwork(Some(banner_url("0").as_str())), Artwork::Loading),
        "a picture not yet asked for is loading"
    );
}

#[test]
fn clearing_forgets_every_picture_and_every_request() {
    let mut page = page_with(3);
    page.wanted(screen());
    page.picture_ready(Asset::Banner(banner_url("0")), Some(picture()));

    page.clear_pictures();

    assert!(page.pictures.is_empty());
    assert!(page.requested.is_empty());
    assert!(page.icon_queue.is_empty());
}
