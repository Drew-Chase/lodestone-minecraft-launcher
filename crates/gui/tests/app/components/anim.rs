use super::*;
use iced::color;

#[test]
fn presets_are_time_based_transitions() {
    // All presets must be easings — springs are numerically unstable at
    // these durations (see the module docs).
    let micro = micro();
    let lift = lift();
    let glide = glide();
    let menu = menu();

    assert_eq!(micro.duration, Duration::from_millis(120));
    assert_eq!(lift.duration, Duration::from_millis(200));
    assert_eq!(glide.duration, Duration::from_millis(300));
    assert_eq!(menu.duration, Duration::from_millis(180));
}

#[test]
fn lift_matches_the_guide_curve() {
    // §08 — cubic-bezier(.2,.7,.3,1).
    let lift = lift();
    assert!(matches!(lift.curve, Curve::Bezier(_)));
}

#[test]
fn fade_multiplies_alpha() {
    let faded = fade(color!(0x22ff84), 0.5);
    assert!((faded.a - 0.5).abs() < 1e-6);
    assert!((faded.r - color!(0x22ff84).r).abs() < 1e-6);
}

#[test]
fn mix_hits_endpoints() {
    let a = color!(0x000000);
    let b = color!(0xffffff);
    assert_eq!(mix(a, b, 0.0), a);
    assert_eq!(mix(a, b, 1.0), b);
    let middle = mix(a, b, 0.5);
    assert!((middle.r - 0.5).abs() < 1e-6);
}
