use super::*;

/// Builds a 64×64 RGBA skin with a red 8×8 face block at (8, 8) and a
/// blue pixel at (100, 30) that must NOT leak into the crop.
fn test_skin() -> Vec<u8> {
    let mut skin = vec![0u8; 64 * 64 * 4];
    for y in 8..16 {
        for x in 8..16 {
            let px = (y * 64 + x) * 4;
            skin[px..px + 4].copy_from_slice(&[255, 0, 0, 255]);
        }
    }
    let px = (30 * 64 + 100) * 4;
    skin[px..px + 4].copy_from_slice(&[0, 0, 255, 255]);
    skin
}

#[test]
fn face_from_rgba_crops_the_face_block() {
    let face = face_from_rgba(&test_skin(), 64).expect("valid skin");
    assert_eq!(face.len(), 8 * 8 * 4);
    // Every cropped pixel is the red block.
    assert!(face.chunks_exact(4).all(|px| px == [255, 0, 0, 255]));
}

#[test]
fn face_from_skin_png_decodes_the_crop() {
    let png = image::RgbaImage::from_raw(64, 64, test_skin()).expect("buffer matches dimensions");
    let mut png_bytes = std::io::Cursor::new(Vec::new());
    png.write_to(&mut png_bytes, image::ImageFormat::Png).expect("encodes");
    let face = face_from_skin_png(png_bytes.get_ref()).expect("valid png");
    assert_eq!(face.len(), 8 * 8 * 4);
    assert!(face.chunks_exact(4).all(|px| px == [255, 0, 0, 255]));
}

#[test]
fn expanded_avatar_is_nearest_neighbour() {
    let mut face = vec![0u8; 8 * 8 * 4];
    // One distinctive pixel: (4, 4) — pure green (the center, away from
    // the rounded-corner mask).
    let center = (4 * 8 + 4) * 4;
    face[center..center + 4].copy_from_slice(&[0, 255, 0, 255]);
    let expanded = expanded_avatar(&face).expect("valid face");
    assert_eq!(expanded.len(), AVATAR_SIZE * AVATAR_SIZE * 4);
    // The central 5×5 block of the 40×40 avatar is that pixel.
    for y in 20..25 {
        for x in 20..25 {
            let px = (y * AVATAR_SIZE + x) * 4;
            assert_eq!(&expanded[px..px + 3], &[0, 255, 0], "pixel ({x},{y})");
        }
    }
    // The pixel next to the block is transparent (the face was black).
    let px = (5 * AVATAR_SIZE) * 4;
    assert_eq!(&expanded[px..px + 4], &[0, 0, 0, 0]);
}

#[test]
fn expanded_avatar_fades_the_corners() {
    let face = vec![255u8; 8 * 8 * 4]; // opaque white face
    let expanded = expanded_avatar(&face).expect("valid face");
    let alpha_at = |x: usize, y: usize| expanded[(y * AVATAR_SIZE + x) * 4 + 3];
    // The center stays fully opaque.
    assert_eq!(alpha_at(20, 20), 255);
    // The extreme corners are fully faded by the rounding mask.
    assert_eq!(alpha_at(0, 0), 0);
    assert_eq!(alpha_at(39, 0), 0);
    assert_eq!(alpha_at(0, 39), 0);
    assert_eq!(alpha_at(39, 39), 0);
}

#[test]
fn the_hat_layer_covers_the_face_where_it_is_opaque() {
    let mut skin = test_skin();
    // An opaque blue hat pixel over the face's top-left texel, at (40, 8).
    let px = (8 * 64 + 40) * 4;
    skin[px..px + 4].copy_from_slice(&[0, 0, 255, 255]);
    let face = face_from_rgba(&skin, 64).expect("valid skin");
    assert_eq!(&face[0..4], &[0, 0, 255, 255], "the hat replaces the face under it");
    assert_eq!(&face[4..8], &[255, 0, 0, 255], "the rest of the face shows through");
}

#[test]
fn a_translucent_hat_pixel_blends_with_the_face() {
    let mut skin = test_skin();
    let px = (8 * 64 + 40) * 4;
    skin[px..px + 4].copy_from_slice(&[0, 0, 255, 128]);
    let face = face_from_rgba(&skin, 64).expect("valid skin");
    // Half blue over opaque red leaves an opaque pixel that is half of each.
    assert_eq!(face[3], 255);
    assert!((i32::from(face[0]) - 127).abs() <= 1, "red {}", face[0]);
    assert_eq!(face[1], 0);
    assert!((i32::from(face[2]) - 128).abs() <= 1, "blue {}", face[2]);
}

#[test]
fn a_skin_too_narrow_for_the_hat_has_no_face() {
    let skin = vec![255u8; 32 * 16 * 4];
    assert!(face_from_rgba(&skin, 32).is_none());
}
