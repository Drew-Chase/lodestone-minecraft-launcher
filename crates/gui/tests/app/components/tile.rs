use super::*;

#[test]
fn a_chosen_tile_rings_in_green_whatever_the_pointer_does() {
    let rest = style(button::Status::Active, 8.0, true);
    let hover = style(button::Status::Hovered, 8.0, true);
    assert_eq!(rest.border.width, 2.0);
    assert_eq!(hover.border.width, 2.0);
    assert_eq!(rest.border.color, color::Primary::DEFAULT);
}

#[test]
fn hovering_adds_a_faint_outline_and_a_lifted_fill() {
    let rest = style(button::Status::Active, 8.0, false);
    let hover = style(button::Status::Hovered, 8.0, false);
    assert!(rest.background.is_none());
    assert!(hover.background.is_some(), "a hovered tile is lifted");
    assert_ne!(rest.border.color, hover.border.color, "a hovered tile is outlined");
    assert_eq!(hover.border.radius, 8.0.into());
}
