//! The capes the player owns, as the page shows them: the front of each cape, which is the side a cape shows when worn.

use iced::widget::image::Handle;

use super::viewer::decode_cape;

/// The size of a cape's thumbnail in pixels: the cape's 10×16 front, four times over.
const THUMBNAIL_WIDTH: u32 = 40;
const THUMBNAIL_HEIGHT: u32 = 64;

/// A cape the player owns, as the page holds it: its id, its front for the list, and its picture for the viewer.
#[derive(Debug, Clone)]
pub struct CapeArt {
    pub id: String,
    pub face: Handle,
    pub png: Vec<u8>,
}

/// The front of a cape: the outside of the cape texture, the 10×16 block at (1, 1), scaled up with nearest sampling so its
/// pixels stay square. `None` when the bytes are not a PNG.
pub fn front_image(png: &[u8]) -> Option<image::RgbaImage> {
    let cape = decode_cape(png)?;
    let front = image::imageops::crop_imm(&cape, 1, 1, 10, 16).to_image();
    Some(image::imageops::resize(
        &front,
        THUMBNAIL_WIDTH,
        THUMBNAIL_HEIGHT,
        image::imageops::FilterType::Nearest,
    ))
}

/// The front of a cape as the picture the page shows.
pub fn front(png: &[u8]) -> Option<Handle> {
    let front = front_image(png)?;
    Some(Handle::from_rgba(THUMBNAIL_WIDTH, THUMBNAIL_HEIGHT, front.into_raw()))
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/skins/capes.rs"]
mod tests;
