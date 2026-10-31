use super::*;

#[test]
fn the_moon_sits_a_third_of_the_way_down() {
    for (width, height) in [(1848.0, 230.0), (388.0, 100.0)] {
        let (scale, _, y) = cover(width, height);
        let moon = MOON_Y * scale + y;
        assert!((moon - height / 3.0).abs() < 0.5, "moon at {moon} in a {width}×{height} frame");
    }
}

#[test]
fn the_scene_covers_the_frame() {
    for (width, height) in [(1848.0, 230.0), (388.0, 100.0), (300.0, 500.0)] {
        let (scale, x, y) = cover(width, height);
        let top = -y / scale;
        assert!(
            x <= 0.0 && SCENE_WIDTH * scale + x >= width - 0.01,
            "{width}×{height} is not covered across"
        );
        assert!(
            top >= 0.0 && top + height / scale <= SCENE_HEIGHT + 0.01,
            "{width}×{height} runs off the scene"
        );
    }
}

#[test]
fn sizes_are_quantised_to_four_pixels() {
    assert_eq!(quantise(388.0), 388);
    assert_eq!(quantise(389.0), 388);
    assert_eq!(quantise(391.0), 392);
    assert_eq!(quantise(0.0), 4);
}

#[test]
fn the_card_art_is_exact_size_with_rounded_top_corners_only() {
    let svg = card_wrapped("", 400.0, 178.0);
    assert!(svg.contains(r#"width="400" height="178""#), "{svg}");
    assert!(svg.contains(r#"d="M0,13 A13,13 0 0 1 13,0 H387 A13,13 0 0 1 400,13 V178 H0 Z""#), "{svg}");
}

/// The hero's clip is drawn in the frame's own pixels, so the cover crop cannot remove its corners.
#[test]
fn the_hero_is_exact_size_and_clipped_in_its_own_pixels() {
    let svg = hero_wrapped("", 1156.0, 300.0);
    assert!(svg.contains(r#"width="1156" height="300""#), "{svg}");
    assert!(svg.contains(r#"<rect width="1156" height="300" rx="18"/>"#), "{svg}");
}
