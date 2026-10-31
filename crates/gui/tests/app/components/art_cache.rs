//! Tests for `art_cache`. They live here rather than inline and reach the private render through `super`.

use super::*;

fn key(biome: Biome, width: f32) -> ArtKey {
    ArtKey::new(Kind::Card, biome, 1, width, 179.0)
}

/// A one-pixel picture, standing in for a finished render.
fn picture() -> Handle {
    Handle::from_rgba(1, 1, vec![0, 0, 0, 255])
}

#[test]
fn widths_snap_to_the_four_pixel_step() {
    assert_eq!(key(Biome::Forest, 341.0), key(Biome::Forest, 339.0));
    assert_ne!(key(Biome::Forest, 341.0), key(Biome::Forest, 349.0));
}

#[test]
fn a_key_is_missing_until_it_renders_and_is_asked_for_once() {
    let mut cache = ArtCache::default();
    let wanted = key(Biome::Forest, 340.0);
    assert_eq!(cache.missing(&[wanted, wanted]), vec![wanted], "a key is listed once");

    let _ = cache.request(vec![wanted], |_, _| ());
    assert!(cache.missing(&[wanted]).is_empty(), "a render in flight is not asked for again");
}

#[test]
fn a_finished_render_is_drawn_and_a_failed_one_is_not_retried() {
    let mut cache = ArtCache::default();
    let good = key(Biome::Forest, 340.0);
    let bad = key(Biome::Ocean, 340.0);
    let _ = cache.request(vec![good, bad], |_, _| ());
    cache.finish(good, Some(picture()));
    cache.finish(bad, None);

    assert!(cache.get(good).is_some());
    assert!(cache.get(bad).is_none());
    assert!(cache.missing(&[bad]).is_empty(), "a failed picture is not rendered again");
}

#[test]
fn the_nearest_width_stands_in_while_another_size_renders() {
    let mut cache = ArtCache::default();
    let rendered = key(Biome::Forest, 340.0);
    let _ = cache.request(vec![rendered], |_, _| ());
    cache.finish(rendered, Some(picture()));

    assert!(
        cache.get(key(Biome::Forest, 360.0)).is_some(),
        "a resize shows the nearest picture at once"
    );
    assert!(cache.get(key(Biome::Ocean, 360.0)).is_none(), "another scene has no stand-in");
}

#[test]
fn a_resize_drops_the_pictures_it_no_longer_needs() {
    let mut cache = ArtCache::default();
    let kept = key(Biome::Forest, 340.0);
    let dropped = key(Biome::Ocean, 340.0);
    let _ = cache.request(vec![kept, dropped], |_, _| ());
    cache.finish(kept, Some(picture()));
    cache.finish(dropped, Some(picture()));

    cache.retain(&HashSet::from([kept]));
    assert!(cache.get(kept).is_some());
    assert!(cache.get(dropped).is_none());
}

#[test]
fn a_render_for_a_key_that_was_dropped_is_ignored() {
    let mut cache = ArtCache::default();
    let wanted = key(Biome::Forest, 340.0);
    let _ = cache.request(vec![wanted], |_, _| ());
    cache.retain(&HashSet::new());

    cache.finish(wanted, Some(picture()));
    assert!(cache.get(wanted).is_none(), "the render no longer belongs to the page");
}

#[test]
fn a_card_renders_to_a_picture() {
    assert!(render(key(Biome::Forest, 340.0)).is_some());
}

#[test]
fn every_kind_renders_a_scene_variant_to_a_picture() {
    for kind in [Kind::Card, Kind::Thumb, Kind::Hero] {
        let picture = render(ArtKey::new(kind, Biome::Cherry, 2, 300.0, 120.0));
        assert!(picture.is_some(), "{kind:?} renders");
    }
}
