//! Project pictures: fetched once, kept on disk, and decoded on the blocking
//! pool, so a picture never holds up a frame. Each picture comes in two forms:
//! the image itself, for thumbnails, and a banner that the picture covers, cropped
//! to the banner's box, for the art of a card or the hero.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use ::image::{ImageFormat, RgbaImage, imageops};
use iced::widget::image;

/// The client pictures are fetched with. Platform CDNs expect a user agent.
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(concat!("lodestone_launcher/", env!("CARGO_PKG_VERSION")))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

/// The banner's width in pixels. Cards and the hero cover it, so the picture keeps its centre
/// whatever the shape of their box.
const BANNER_WIDTH: u32 = 1200;
/// The banner's height in pixels.
const BANNER_HEIGHT: u32 = 400;
/// The side of an instance's icon in pixels. Instance icons are drawn small, so this is ample.
const ICON_SIDE: u32 = 256;

/// A project's picture in the two forms the page shows.
#[derive(Debug, Clone)]
pub struct Picture {
    /// The picture as the platform gave it, for thumbnails.
    pub icon: image::Handle,
    /// The picture scaled to cover the banner's box, for card and hero art.
    pub banner: image::Handle,
}

/// Where a downloaded picture is kept: `<cache dir>/lodestone/discover-art/<hash of the url>`.
fn cache_path(url: &str) -> Option<PathBuf> {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    Some(
        dirs::cache_dir()?
            .join("lodestone")
            .join("discover-art")
            .join(format!("{:016x}", hasher.finish())),
    )
}

/// Loads the picture at `url`: from the disk cache when it is there, otherwise over
/// the network, and then into the cache. `None` when it cannot be had or decoded.
pub async fn load(url: String) -> Option<Picture> {
    let bytes = fetch(&url).await?;
    tokio::task::spawn_blocking(move || decode(&bytes)).await.ok().flatten()
}

/// The picture at `url` as it is, for pictures shown whole: readme images, gallery thumbnails and avatars. It has no
/// banner form, so it costs one decode. `None` when it cannot be had or decoded.
pub async fn load_image(url: String) -> Option<image::Handle> {
    let bytes = fetch(&url).await?;
    tokio::task::spawn_blocking(move || decode_image(&bytes)).await.ok().flatten()
}

/// The picture decoded as it is.
fn decode_image(bytes: &[u8]) -> Option<image::Handle> {
    let picture = ::image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = picture.dimensions();
    Some(image::Handle::from_rgba(width, height, picture.into_raw()))
}

/// The picture's bytes: from the disk cache when they are there, otherwise over the network, and then into
/// the cache. `None` when they cannot be had.
async fn fetch(url: &str) -> Option<Vec<u8>> {
    let path = cache_path(url);
    let cached = match path.clone() {
        Some(path) => tokio::task::spawn_blocking(move || std::fs::read(path).ok()).await.ok().flatten(),
        None => None,
    };
    match cached {
        Some(bytes) => Some(bytes),
        None => {
            let bytes = download(url).await?;
            if let Some(path) = path {
                let copy = bytes.clone();
                let _ = tokio::task::spawn_blocking(move || write_cache(&path, &copy)).await;
            }
            Some(bytes)
        }
    }
}

async fn download(url: &str) -> Option<Vec<u8>> {
    let response = CLIENT.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    Some(response.bytes().await.ok()?.to_vec())
}

/// Writes a cache entry. A cache that cannot be written only costs another download.
fn write_cache(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, bytes);
}

/// The art an instance keeps from a pack: its icon as a square PNG, and its banner as a PNG.
#[derive(Debug, Clone)]
pub struct PackArt {
    pub icon: Vec<u8>,
    pub banner: Vec<u8>,
}

/// The pack's picture at `url`, encoded the way an instance keeps its art. `None` when the picture cannot be
/// had or decoded.
pub async fn pack_art(url: String) -> Option<PackArt> {
    let bytes = fetch(&url).await?;
    tokio::task::spawn_blocking(move || encode_pack_art(&bytes)).await.ok().flatten()
}

/// Builds an instance's icon, a centred square of the picture, and its banner, both as PNG.
fn encode_pack_art(bytes: &[u8]) -> Option<PackArt> {
    let picture = ::image::load_from_memory(bytes).ok()?.to_rgba8();
    if picture.width() == 0 || picture.height() == 0 {
        return None;
    }
    Some(PackArt {
        icon: encode_png(&square(&picture))?,
        banner: encode_png(&banner(&picture))?,
    })
}

/// The picture's centred square, scaled to an instance icon's side.
fn square(picture: &RgbaImage) -> RgbaImage {
    let side = picture.width().min(picture.height());
    let left = (picture.width() - side) / 2;
    let top = (picture.height() - side) / 2;
    let cropped = imageops::crop_imm(picture, left, top, side, side).to_image();
    imageops::resize(&cropped, ICON_SIDE, ICON_SIDE, imageops::FilterType::Lanczos3)
}

/// The image encoded as PNG.
fn encode_png(image: &RgbaImage) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    image.write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png).ok()?;
    Some(bytes)
}

/// Decodes the picture and builds both of its forms. `None` for a picture with no pixels, which has no banner to cover.
fn decode(bytes: &[u8]) -> Option<Picture> {
    let picture = ::image::load_from_memory(bytes).ok()?.to_rgba8();
    if picture.width() == 0 || picture.height() == 0 {
        return None;
    }
    let banner = banner(&picture);
    let (width, height) = picture.dimensions();
    Some(Picture {
        icon: image::Handle::from_rgba(width, height, picture.into_raw()),
        banner: image::Handle::from_rgba(BANNER_WIDTH, BANNER_HEIGHT, banner.into_raw()),
    })
}

/// The banner: the picture scaled to cover the banner's box, keeping its proportions, with the middle of it kept and
/// its edges cropped away. Nothing is stretched and no backdrop is drawn, so it is the platform's art as it is.
fn banner(picture: &RgbaImage) -> RgbaImage {
    let (width, height) = covering(picture.width(), picture.height(), BANNER_WIDTH, BANNER_HEIGHT);
    let scaled = imageops::resize(picture, width, height, imageops::FilterType::Lanczos3);
    imageops::crop_imm(
        &scaled,
        (width - BANNER_WIDTH) / 2,
        (height - BANNER_HEIGHT) / 2,
        BANNER_WIDTH,
        BANNER_HEIGHT,
    )
    .to_image()
}

/// The smallest size that keeps a `width` by `height` picture's proportions and covers a `box_width` by `box_height`
/// box. Neither side is less than the box's, so a centred crop always fits.
fn covering(width: u32, height: u32, box_width: u32, box_height: u32) -> (u32, u32) {
    let scale = (f64::from(box_width) / f64::from(width)).max(f64::from(box_height) / f64::from(height));
    (
        ((f64::from(width) * scale).ceil() as u32).max(box_width),
        ((f64::from(height) * scale).ceil() as u32).max(box_height),
    )
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/discover/images.rs"]
mod tests;
