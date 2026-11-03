use super::*;

/// A 64×32 cape PNG, with each listed texel painted and the rest clear.
fn cape_png(painted: &[(u32, u32, [u8; 4])]) -> Vec<u8> {
    let mut image = image::RgbaImage::new(64, 32);
    for &(x, y, color) in painted {
        image.put_pixel(x, y, image::Rgba(color));
    }
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).expect("encodes");
    bytes.into_inner()
}

#[test]
fn the_front_is_the_outside_of_the_cape_scaled_four_times() {
    let red = [255, 0, 0, 255];
    let blue = [0, 0, 255, 255];
    let green = [0, 255, 0, 255];
    // The outside's corners are red and blue. The inside (x from 12) is green, and must not show.
    let png = cape_png(&[(1, 1, red), (10, 16, blue), (12, 1, green)]);
    let front = front_image(&png).expect("valid cape");
    assert_eq!(front.dimensions(), (40, 64));
    assert_eq!(front.get_pixel(0, 0).0, red, "the outside's top-left texel is the thumbnail's top-left");
    assert_eq!(front.get_pixel(39, 63).0, blue, "the outside's bottom-right texel is its bottom-right");
    assert!(
        front.pixels().all(|pixel| pixel.0 != green),
        "the inside of the cape is not part of the front"
    );
}

#[test]
fn bytes_that_are_not_a_cape_have_no_front() {
    assert!(front(b"not a cape").is_none());
}
