//! Tests for `scale`. The factor is one global, so a single test sets it and puts it back.

use super::*;

#[test]
fn device_pixels_follow_the_factor_and_bad_values_are_ignored() {
    set(2.0);
    assert_eq!(device_px(44.0), 88);
    assert_eq!(device_px(36.0), 72);

    set(1.5);
    assert_eq!(device_px(36.0), 54);
    assert_eq!(device_px(0.2), 1, "a box is never smaller than one pixel");

    set(0.0);
    set(f32::NAN);
    set(-2.0);
    assert_eq!(factor(), 1.5, "zero, NaN and negative factors are ignored");

    set(1.0);
    assert_eq!(device_px(44.0), 44);
}
