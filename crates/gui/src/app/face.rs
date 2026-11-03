//! Player face extraction — the 8×8 helm crop out of a Minecraft skin.
//!
//! A skin texture is a 64×64 PNG whose face occupies the 8×8 block at
//! (8, 8), with the hat layer (the helm) over it at (40, 8). This module
//! turns skin PNG bytes (downloaded from Mojang) into that helm as RGBA, and
//! expands it to a crisp 40×40 texture for the sidebar avatar
//! (nearest-neighbour, so the pixels stay square).

use image::ImageReader;
use image::RgbaImage;

/// The face's side in pixels on the skin texture.
const FACE_SIZE: usize = 8;
/// Where the face sits on the skin texture (both axes).
const FACE_OFFSET: usize = 8;
/// Where the hat layer sits on the skin texture, across. Its rows share the face's.
const HAT_OFFSET_X: usize = 40;
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

/// Crops the helm out of raw skin RGBA (64×64×4, row-major): the face, with the hat layer drawn over it. A skin with no
/// hat has transparent pixels there, so its crop is the face alone.
pub fn face_from_rgba(skin_rgba: &[u8], skin_width: usize) -> Option<Vec<u8>> {
    let stride = skin_width * 4;
    if skin_width < HAT_OFFSET_X + FACE_SIZE || skin_rgba.len() < stride * (FACE_OFFSET + FACE_SIZE) {
        return None;
    }
    let mut face = Vec::with_capacity(FACE_SIZE * FACE_SIZE * 4);
    for y in 0..FACE_SIZE {
        let row = (FACE_OFFSET + y) * stride;
        for x in 0..FACE_SIZE {
            let base = row + (FACE_OFFSET + x) * 4;
            let hat = row + (HAT_OFFSET_X + x) * 4;
            face.extend_from_slice(&over(&skin_rgba[base..base + 4], &skin_rgba[hat..hat + 4]));
        }
    }
    Some(face)
}

/// `top` drawn over `bottom`, both straight (not premultiplied) RGBA, with source-over alpha.
fn over(bottom: &[u8], top: &[u8]) -> [u8; 4] {
    let top_alpha = u32::from(top[3]);
    if top_alpha == 0 {
        return [bottom[0], bottom[1], bottom[2], bottom[3]];
    }
    let bottom_alpha = u32::from(bottom[3]);
    // The alpha that is left of the bottom once the top covers its share of the pixel.
    let uncovered = bottom_alpha * (255 - top_alpha) / 255;
    let out_alpha = top_alpha + uncovered;
    if out_alpha == 0 {
        return [0, 0, 0, 0];
    }
    let channel = |index: usize| (u32::from(top[index]) * top_alpha + u32::from(bottom[index]) * uncovered) / out_alpha;
    [channel(0) as u8, channel(1) as u8, channel(2) as u8, out_alpha as u8]
}

/// Nearest-neighbour expansion of an 8×8 RGBA face to the avatar size —
/// keeps the pixels square and crisp (the GPU's linear sampling would blur
/// them) — then fades the corners out along a rounded-rect mask
/// (radius [`AVATAR_RADIUS`]) so the face matches the UI's rounded chips.
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
    apply_rounded_alpha(&mut out, AVATAR_SIZE as f32, AVATAR_RADIUS);
    Some(out)
}

/// The avatar's corner rounding, in pixels.
pub const AVATAR_RADIUS: f32 = 8.0;

/// Multiplies each pixel's alpha by a rounded-rect coverage mask
/// (signed-distance based, ~1px anti-aliased edge).
fn apply_rounded_alpha(rgba: &mut [u8], size: f32, radius: f32) {
    let half = size / 2.0;
    let inset = half - radius;
    for y in 0..size as usize {
        for x in 0..size as usize {
            let px = ((x as f32 + 0.5) - half, (y as f32 + 0.5) - half);
            let qx = (px.0.abs() - inset).max(0.0);
            let qy = (px.1.abs() - inset).max(0.0);
            let dist = qx.hypot(qy) + (qx.max(0.0).min(qy.max(0.0))) - radius;
            let coverage = (0.5 - dist).clamp(0.0, 1.0);
            let index = (y * size as usize + x) * 4 + 3;
            rgba[index] = (rgba[index] as f32 * coverage) as u8;
        }
    }
}

#[cfg(test)]
#[path = "../../tests/app/face.rs"]
mod tests;
