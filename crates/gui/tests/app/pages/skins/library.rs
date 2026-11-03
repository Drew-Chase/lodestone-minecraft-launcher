use super::*;

/// A PNG of a `width`×`height` image filled with one colour.
fn png(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba(color));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).expect("encodes");
    bytes.into_inner()
}

#[test]
fn the_size_is_read_from_the_png_header() {
    assert_eq!(dimensions(&png(64, 32, [0, 0, 0, 255])), Some((64, 32)));
    assert_eq!(dimensions(b"not a picture"), None);
}

#[test]
fn the_accent_is_the_most_common_colour_rounded_to_four_bits() {
    // Four bits per channel: 200 is bucket 12, shown as 12 × 17 = 204 (0xcc), and 30 is bucket 1, shown as 17 (0x11).
    assert_eq!(accent_hex(&png(4, 4, [200, 30, 30, 255])).as_deref(), Some("#cc1111"));
}

#[test]
fn near_black_and_transparent_pixels_do_not_stand_for_the_skin() {
    assert_eq!(accent_hex(&png(4, 4, [10, 10, 10, 255])), None, "near-black pixels are skipped");
    assert_eq!(accent_hex(&png(4, 4, [200, 30, 30, 0])), None, "transparent pixels are skipped");
}

#[test]
fn the_digest_is_the_sha1_of_the_bytes() {
    assert_eq!(digest(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
}

#[test]
fn a_saved_skin_is_worked_out_once_from_its_bytes() {
    let bytes = png(64, 64, [200, 30, 30, 255]);
    let skin = SavedSkin::new("Red".to_string(), PathBuf::from("Red.png"), bytes.clone(), None);
    assert_eq!(skin.size, Some((64, 64)));
    assert_eq!(skin.accent.as_deref(), Some("#cc1111"));
    assert_eq!(skin.digest, digest(&bytes));
    assert_eq!(skin.slim, None);
}
