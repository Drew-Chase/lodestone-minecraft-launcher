//! Tests for `components/button.rs`. They live here rather than inline and
//! are compiled as the `tests` child module of `button`, so they can reach
//! the private `visual` helper.

use std::time::Duration;

use super::*;

fn button<'a>() -> TextButton<'a, ()> {
    TextButton::primary("Play")
}

#[test]
fn hover_lifts_and_brightens() {
    let idle = button().visual(Status::Idle);
    let hovered = button().visual(Status::Hovered);
    assert_eq!(idle.lift, 0.0);
    assert_eq!(hovered.lift, 1.0);
    assert_ne!(idle.background, hovered.background);
}

#[test]
fn disabled_has_no_lift() {
    for status in [Status::Idle, Status::Hovered, Status::Pressed] {
        assert_eq!(button().disabled(true).visual(status).lift, 0.0);
    }
}

#[test]
fn ghost_is_quieter_than_primary() {
    let primary = TextButton::<()>::primary("Play").visual(Status::Idle);
    let ghost = TextButton::<()>::ghost("Edit").visual(Status::Idle);
    assert!(primary.glow > ghost.glow);
    assert_ne!(primary.background, ghost.background);
}

#[test]
fn animation_mode_is_liftable() {
    // The preset must be constructible — guards against API drift.
    let _mode: Mode = anim::lift().into();
    let _ = Duration::from_millis(200);
}

#[test]
fn danger_is_a_tint_at_rest_and_solid_on_hover() {
    let idle = TextButton::<()>::primary("Delete").variant(Variant::Danger).visual(Status::Idle);
    let hovered = TextButton::<()>::primary("Delete").variant(Variant::Danger).visual(Status::Hovered);
    assert!(idle.background.a < 0.5, "a resting danger button is a tint, not a solid fill");
    assert_eq!(hovered.background, DANGER);
    assert_eq!(hovered.lift, 1.0);
}
