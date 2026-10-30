use super::*;
use iced::{Background, Color};

/// A status with the pointer over neither bar.
fn active() -> Status {
    Status::Active {
        is_horizontal_scrollbar_disabled: false,
        is_vertical_scrollbar_disabled: false,
    }
}

/// A status with the pointer over the content, and over the vertical bar only if `over_bar`.
fn hovered(over_bar: bool) -> Status {
    Status::Hovered {
        is_horizontal_scrollbar_hovered: false,
        is_vertical_scrollbar_hovered: over_bar,
        is_horizontal_scrollbar_disabled: false,
        is_vertical_scrollbar_disabled: false,
    }
}

/// A status with the vertical thumb being dragged.
fn dragged() -> Status {
    Status::Dragged {
        is_horizontal_scrollbar_dragged: false,
        is_vertical_scrollbar_dragged: true,
        is_horizontal_scrollbar_disabled: false,
        is_vertical_scrollbar_disabled: false,
    }
}

/// The vertical thumb's colour under `status`.
fn thumb(status: Status) -> Color {
    match style(&Theme::Dark, status).vertical_rail.scroller.background {
        Background::Color(color) => color,
        _ => panic!("the thumb is a solid colour"),
    }
}

#[test]
fn the_thumb_sits_five_pixels_in_from_each_side_of_the_track() {
    assert_eq!((TRACK_WIDTH - THUMB_WIDTH) / 2.0, 5.0);
}

#[test]
fn the_thumb_rests_at_half_opacity() {
    let rest = thumb(active());
    assert_eq!((rest.r, rest.g, rest.b), (1.0, 1.0, 1.0));
    assert!((rest.a - 0.5).abs() < 1e-6);
}

#[test]
fn hovering_the_bar_brings_the_thumb_to_full_opacity() {
    assert!((thumb(hovered(true)).a - 1.0).abs() < 1e-6);
}

#[test]
fn hovering_the_content_alone_leaves_the_thumb_dim() {
    assert!((thumb(hovered(false)).a - 0.5).abs() < 1e-6);
}

#[test]
fn dragging_the_thumb_keeps_it_at_full_opacity() {
    assert!((thumb(dragged()).a - 1.0).abs() < 1e-6);
}

#[test]
fn the_track_has_no_fill() {
    assert_eq!(style(&Theme::Dark, active()).vertical_rail.background, None);
}

#[test]
fn the_thumb_is_fully_rounded() {
    let radius = style(&Theme::Dark, active()).vertical_rail.scroller.border.radius;
    assert_eq!(radius, border::rounded(u32::MAX).radius);
}
