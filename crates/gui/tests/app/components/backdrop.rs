use super::*;

#[test]
fn aurora_offset_hits_the_css_keyframes() {
    let (dx, dy, scale) = aurora_offset(0.0);
    assert_eq!((dx, dy, scale), (0.0, 0.0, 1.0));
    let (dx, dy, scale) = aurora_offset(1.0 / 3.0);
    assert!((dx - 60.0).abs() < 1e-4 && (dy + 40.0).abs() < 1e-4 && (scale - 1.1).abs() < 1e-4);
    let (dx, dy, scale) = aurora_offset(2.0 / 3.0);
    assert!((dx + 40.0).abs() < 1e-4 && (dy - 50.0).abs() < 1e-4 && (scale - 0.95).abs() < 1e-4);
    let (dx, dy, scale) = aurora_offset(1.0);
    assert_eq!((dx, dy, scale), (0.0, 0.0, 1.0));
}

#[test]
fn ember_layout_matches_the_reference_field() {
    let (left, size, delay, dur) = ember_layout(0);
    assert_eq!((left, size, delay), (0.0, 1.0, 0.0));
    assert!((8.0..14.0).contains(&dur));
    let (left, size, ..) = ember_layout(1);
    assert!((0.0..100.0).contains(&left), "left must be a percentage");
    assert!((1.0..=3.0).contains(&size));
    // Two embers never share the same spawn column.
    assert_ne!(ember_layout(0).0, ember_layout(3).0);
}

#[test]
fn scene_svg_carries_every_layer() {
    let svg = scene_svg(1280.0, 820.0);
    assert!(svg.contains("url(#base)"), "flat dark-green base");
    assert!(svg.contains("url(#dust)"), "pixel-dust texture");
    assert!(svg.contains("url(#vignette)"), "corner falloff");
    // Full-window filters cost seconds of CPU on first raster — the
    // scene must stay filter-free.
    assert!(!svg.contains("feTurbulence"), "no turbulence filters");
    assert!(!svg.contains("feGaussianBlur"), "no per-scene blur");
    assert!(!svg.contains("<polygon"), "no terrain silhouettes");
}

#[test]
fn blobs_are_authored_precompensated() {
    let green = blob_svg("#22ff84", 0.59);
    assert!(green.contains("stop-color=\"#22ff84\""));
    assert!(green.contains("stop-opacity=\"0.59\""));
    assert!(green.contains("stop-opacity=\"0.32\""), "mid stop at 55% of peak");
    let amber = blob_svg("#ffcf7d", 0.50);
    assert!(amber.contains("stop-color=\"#ffcf7d\""));
}
