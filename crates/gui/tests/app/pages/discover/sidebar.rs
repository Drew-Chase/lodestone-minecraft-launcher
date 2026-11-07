use super::*;

/// The fill a row drawn in `status` gets, or `None` for a row with no fill.
fn fill(picked: bool, status: button::Status) -> Option<Color> {
    match row_style(picked, status).background {
        Some(Background::Color(paint)) => Some(paint),
        _ => None,
    }
}

#[test]
fn an_idle_row_has_no_fill_so_its_card_shows_through() {
    assert_eq!(fill(false, button::Status::Active), None);
}

#[test]
fn a_hovered_row_is_lighter_than_its_card() {
    assert_eq!(fill(false, button::Status::Hovered), Some(color::surface_bg4()));
}

#[test]
fn a_picked_row_is_tinted_in_the_accent() {
    assert_eq!(fill(true, button::Status::Active), Some(color::primary_tint()));
}

#[test]
fn a_picked_row_deepens_its_tint_under_the_pointer() {
    let tint = fill(true, button::Status::Active).expect("a picked row is tinted");
    let hovered = fill(true, button::Status::Hovered).expect("a picked row under the pointer is tinted");
    assert!(hovered.a > tint.a, "the pointer deepens the accent tint");
}
