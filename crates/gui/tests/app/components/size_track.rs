use super::*;

#[test]
fn a_size_the_view_already_knows_is_not_reported() {
    assert_eq!(changed(Size::new(640.0, 400.0), Size::new(640.0, 400.0)), None);
    assert_eq!(
        changed(Size::new(640.2, 399.8), Size::new(640.0, 400.0)),
        None,
        "a rounding error is not a resize"
    );
}

#[test]
fn a_new_width_or_height_is_reported() {
    assert_eq!(changed(Size::new(700.0, 400.0), Size::new(640.0, 400.0)), Some(Size::new(700.0, 400.0)));
    assert_eq!(changed(Size::new(640.0, 300.0), Size::new(640.0, 400.0)), Some(Size::new(640.0, 300.0)));
    assert_eq!(
        changed(Size::new(640.0, 400.0), Size::ZERO),
        Some(Size::new(640.0, 400.0)),
        "the first layout"
    );
}
