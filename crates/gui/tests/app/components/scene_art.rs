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
