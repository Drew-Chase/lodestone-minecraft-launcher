//! Uploaded art: picking an image, then turning it into the square icon or the 16:9
//! banner an instance keeps. Decoding is CPU work, so it runs on a blocking thread.

use std::io::Cursor;

use image::ImageFormat;
use image::imageops::FilterType;

/// The largest file taken. Anything bigger is refused before it is decoded.
const MAX_BYTES: usize = 25 * 1024 * 1024;
/// The icon's edge, in pixels.
const ICON_EDGE: u32 = 256;
/// The banner's frame, in pixels: 16:9.
const BANNER_WIDTH: u32 = 1280;
const BANNER_HEIGHT: u32 = 720;

/// Asks for an icon image and returns it as a square PNG. `None` when the dialog is
/// cancelled.
pub async fn pick_icon() -> Result<Option<Vec<u8>>, String> {
    match pick().await? {
        Some(bytes) => encode(bytes, ICON_EDGE, ICON_EDGE).await.map(Some),
        None => Ok(None),
    }
}

/// Asks for a banner image and returns it as a 16:9 PNG. `None` when the dialog is
/// cancelled.
pub async fn pick_banner() -> Result<Option<Vec<u8>>, String> {
    match pick().await? {
        Some(bytes) => encode(bytes, BANNER_WIDTH, BANNER_HEIGHT).await.map(Some),
        None => Ok(None),
    }
}

/// The file the user picks, read into memory.
async fn pick() -> Result<Option<Vec<u8>>, String> {
    let Some(file) = rfd::AsyncFileDialog::new()
        .set_title("Choose an image")
        .add_filter("Images", &["png", "jpg", "jpeg"])
        .pick_file()
        .await
    else {
        return Ok(None);
    };
    let bytes = file.read().await;
    if bytes.len() > MAX_BYTES {
        return Err("That image is larger than 25 MB.".to_string());
    }
    Ok(Some(bytes))
}

/// Decodes `bytes`, covers a `width` by `height` frame with it, and encodes the result as PNG.
async fn encode(bytes: Vec<u8>, width: u32, height: u32) -> Result<Vec<u8>, String> {
    tokio::task::spawn_blocking(move || {
        let decoded = image::load_from_memory(&bytes).map_err(|error| format!("could not read that image: {error}"))?;
        let framed = decoded.resize_to_fill(width, height, FilterType::Lanczos3);
        let mut png = Cursor::new(Vec::new());
        framed
            .write_to(&mut png, ImageFormat::Png)
            .map_err(|error| format!("could not encode the image: {error}"))?;
        Ok(png.into_inner())
    })
    .await
    .map_err(|error| error.to_string())?
}
