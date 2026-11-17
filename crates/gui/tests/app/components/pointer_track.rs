use super::*;

#[test]
fn the_pointer_is_reported_relative_to_the_widgets_corner() {
    assert_eq!(relative(Point::new(340.0, 260.0), Point::new(300.0, 200.0)), Point::new(40.0, 60.0));
}

#[test]
fn a_pointer_left_of_or_above_the_widget_has_a_negative_offset() {
    // A drag may leave the list, and the label that follows the pointer is clamped by the view, not here.
    assert_eq!(relative(Point::new(10.0, 5.0), Point::new(30.0, 20.0)), Point::new(-20.0, -15.0));
}
