//! Tests for `boot.rs`. They live here rather than inline and are compiled as the `tests` child
//! module of `boot`, so they can reach the private icon helpers.

use super::icon_pixels;

#[test]
fn the_application_icon_is_the_512_pixel_mark() {
    let (rgba, width, height) = icon_pixels().expect("the embedded icon decodes");
    assert_eq!((width, height), (512, 512));
    assert_eq!(rgba.len(), 512 * 512 * 4);
}
