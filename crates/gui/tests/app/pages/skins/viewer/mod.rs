use super::*;

/// A PNG of a `width`×`height` image filled with one colour.
fn png(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba(color));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).expect("encodes");
    bytes.into_inner()
}

/// The texel at `(x, y)` of the texture the viewer samples.
fn texel(skin: &Skin, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * ATLAS_WIDTH + x) * 4) as usize;
    [skin.texels[at], skin.texels[at + 1], skin.texels[at + 2], skin.texels[at + 3]]
}

#[test]
fn the_skin_fills_the_left_half_of_the_texture() {
    let skin = Skin::compose(&png(64, 64, [255, 0, 0, 255]), None).expect("valid skin");
    assert_eq!(texel(&skin, 0, 0), [255, 0, 0, 255]);
    assert_eq!(texel(&skin, 63, 63), [255, 0, 0, 255]);
    assert_eq!(texel(&skin, 64, 0), [0, 0, 0, 0], "no cape, so the right half stays clear");
    assert!(!skin.cape);
}

#[test]
fn a_worn_cape_is_packed_beside_the_skin() {
    let skin = Skin::compose(&png(64, 64, [255, 0, 0, 255]), Some(&png(64, 32, [0, 0, 255, 255]))).expect("valid skin");
    assert!(skin.cape);
    assert_eq!(texel(&skin, 0, 0), [255, 0, 0, 255], "the skin keeps its place");
    assert_eq!(texel(&skin, 64, 0), [0, 0, 255, 255], "the cape starts at the middle of the texture");
    assert_eq!(texel(&skin, 127, 31), [0, 0, 255, 255]);
    assert_eq!(texel(&skin, 64, 32), [0, 0, 0, 0], "a cape is 32 texels tall");
}

#[test]
fn a_cape_of_another_size_is_resampled_to_the_standard_shape() {
    let cape = decode_cape(&png(128, 64, [0, 255, 0, 255])).expect("valid cape");
    assert_eq!(cape.dimensions(), (64, 32));
}

#[test]
fn a_cape_changes_the_texture_id() {
    let plain = Skin::compose(&png(64, 64, [255, 0, 0, 255]), None).expect("valid skin");
    let caped = Skin::compose(&png(64, 64, [255, 0, 0, 255]), Some(&png(64, 32, [0, 0, 255, 255]))).expect("valid skin");
    assert_ne!(plain.id, caped.id, "the GPU texture uploads again when the cape changes");
}

#[test]
fn a_skin_of_the_wrong_size_is_refused() {
    assert!(Skin::compose(&png(32, 32, [0, 0, 0, 255]), None).is_none());
}
