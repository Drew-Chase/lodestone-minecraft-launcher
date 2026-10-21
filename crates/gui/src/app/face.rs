//! Player face extraction — the 8×8 face crop out of a Minecraft skin.
//!
//! A skin texture is a 64×64 PNG whose face occupies the 8×8 block at
//! (8, 8). This module turns skin PNG bytes (downloaded from Mojang) into
//! that face as RGBA, and expands it to a crisp 40×40 texture for the
//! sidebar avatar (nearest-neighbour, so the pixels stay square).

use image::ImageReader;
use image::RgbaImage;

/// The face's side in pixels on the skin texture.
const FACE_SIZE: usize = 8;
/// Where the face sits on the skin texture (both axes).
const FACE_OFFSET: usize = 8;
/// The avatar's rendered size — 5× per face pixel.
pub const AVATAR_SIZE: usize = 40;

/// Crops the 8×8 face out of a skin PNG, returning RGBA bytes
/// (`FACE_SIZE * FACE_SIZE * 4`).
pub fn face_from_skin_png(skin_png: &[u8]) -> Option<Vec<u8>> {
    let skin: RgbaImage = ImageReader::new(std::io::Cursor::new(skin_png))
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?
        .to_rgba8();
    face_from_rgba(&skin.clone().into_raw(), skin.width() as usize)
}

/// Crops the face out of raw skin RGBA (64×64×4, row-major).
pub fn face_from_rgba(skin_rgba: &[u8], skin_width: usize) -> Option<Vec<u8>> {
    let stride = skin_width * 4;
    if skin_rgba.len() < stride * (FACE_OFFSET + FACE_SIZE) {
        return None;
    }
    let mut face = Vec::with_capacity(FACE_SIZE * FACE_SIZE * 4);
    for y in 0..FACE_SIZE {
        for x in 0..FACE_SIZE {
            let px = ((FACE_OFFSET + y) * stride) + (FACE_OFFSET + x) * 4;
            face.extend_from_slice(&skin_rgba[px..px + 4]);
        }
    }
    Some(face)
}

/// Nearest-neighbour expansion of an 8×8 RGBA face to the avatar size —
/// keeps the pixels square and crisp (the GPU's linear sampling would blur
/// them).
pub fn expanded_avatar(face: &[u8]) -> Option<Vec<u8>> {
    if face.len() != FACE_SIZE * FACE_SIZE * 4 {
        return None;
    }
    let scale = AVATAR_SIZE / FACE_SIZE;
    let mut out = Vec::with_capacity(AVATAR_SIZE * AVATAR_SIZE * 4);
    for y in 0..AVATAR_SIZE {
        let source_y = (y / scale) * FACE_SIZE * 4;
        for x in 0..AVATAR_SIZE {
            let source_x = (x / scale) * 4;
            let px = source_y + source_x;
            out.extend_from_slice(&face[px..px + 4]);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
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
        // One distinctive pixel: (0, 0) — pure green.
        face[0..4].copy_from_slice(&[0, 255, 0, 255]);
        let expanded = expanded_avatar(&face).expect("valid face");
        assert_eq!(expanded.len(), AVATAR_SIZE * AVATAR_SIZE * 4);
        // The top-left 5×5 block of the 40×40 avatar is that pixel.
        for y in 0..5 {
            for x in 0..5 {
                let px = (y * AVATAR_SIZE + x) * 4;
                assert_eq!(&expanded[px..px + 4], &[0, 255, 0, 255], "pixel ({x},{y})");
            }
        }
        // The pixel next to the block is transparent (the face was black).
        let px = (5 * AVATAR_SIZE) * 4;
        assert_eq!(&expanded[px..px + 4], &[0, 0, 0, 0]);
    }
}
