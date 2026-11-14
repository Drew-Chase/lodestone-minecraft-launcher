//! Project pictures: fetched once, kept on disk, and decoded on the blocking
//! pool, so a picture never holds up a frame. Each form of a picture is an
//! [`Asset`] that loads on its own:
//!
//! - the banner, the picture cut to cover the banner's box, for the art of a card or the hero;
//! - the icon, fitted to the square it is drawn in at the window's scale, and kept on disk at that size, so a later load
//!   decodes only the small copy;
//! - the picture as it is, for pictures shown whole: gallery, readme, news and the viewer.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use ::image::{ImageFormat, RgbaImage, imageops};
use iced::widget::image;

/// The client pictures are fetched with. Platform CDNs expect a user agent.
static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    lodestone_net::builder()
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

/// One picture a page draws. Each form is fetched, decoded and kept on its own, so the map of loaded pictures is keyed by it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Asset {
    /// The picture as the platform gave it, drawn whole.
    Native(String),
    /// The banner: the picture covering the banner's box.
    Banner(String),
    /// The icon, fitted within a square of `px` device pixels.
    Icon { url: String, px: u32 },
}

/// Where a downloaded picture is kept: `discover-art/<hash of the url>` in the launcher's cache folder.
fn cache_path(url: &str) -> Option<PathBuf> {
    cache_file(url, "")
}

/// Where the copy of a picture at one icon size is kept: `discover-art/<hash of the url>-<px>.png`, beside the original.
fn icon_cache_path(url: &str, px: u32) -> Option<PathBuf> {
    cache_file(url, &format!("-{px}.png"))
}

/// A file in the picture cache, named by the hash of `url` and `suffix`.
fn cache_file(url: &str, suffix: &str) -> Option<PathBuf> {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    Some(
        crate::app::paths::cache_dir()?
            .join("discover-art")
            .join(format!("{:016x}{suffix}", hasher.finish())),
    )
}

/// Loads the banner of the picture at `url`: from the disk cache when it is there, otherwise over the network, and then into
/// the cache. `None` when it cannot be had or decoded.
pub async fn load_banner(url: String) -> Option<image::Handle> {
    let bytes = fetch(&url).await?;
    tokio::task::spawn_blocking(move || decode_banner(&bytes)).await.ok().flatten()
}

/// Loads the icon of the picture at `url`, fitted within a `px` square of device pixels. The copy at that size is kept on disk
/// beside the original, so a later load decodes only the copy. `None` when the picture cannot be had or decoded.
pub async fn load_icon(url: String, px: u32) -> Option<image::Handle> {
    let copy = icon_cache_path(&url, px);
    if let Some(path) = copy.clone() {
        let kept = tokio::task::spawn_blocking(move || std::fs::read(path).ok()).await.ok().flatten();
        if let Some(bytes) = kept {
            let handle = tokio::task::spawn_blocking(move || decode_image(&bytes)).await.ok().flatten();
            if handle.is_some() {
                return handle;
            }
        }
    }
    let bytes = fetch(&url).await?;
    tokio::task::spawn_blocking(move || {
        let (handle, png) = fit_icon(&bytes, px)?;
        if let Some(path) = copy {
            write_cache(&path, &png);
        }
        Some(handle)
    })
    .await
    .ok()
    .flatten()
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

/// Writes a cache entry atomically, so a reader never sees half of one. A cache that cannot be written only costs another
/// download.
fn write_cache(path: &Path, bytes: &[u8]) {
    let _ = lodestone_fs::write_atomic(path, bytes);
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

/// Decodes the picture and cuts it to the banner's box. `None` for a picture with no pixels, which has no banner to cover.
fn decode_banner(bytes: &[u8]) -> Option<image::Handle> {
    let picture = ::image::load_from_memory(bytes).ok()?.to_rgba8();
    if picture.width() == 0 || picture.height() == 0 {
        return None;
    }
    let banner = banner(&picture);
    Some(image::Handle::from_rgba(BANNER_WIDTH, BANNER_HEIGHT, banner.into_raw()))
}

/// Decodes an icon and fits it within a `px` square. It returns the handle to draw and the PNG to keep on disk, which hold the
/// same picture. A picture already within the square keeps its size, since enlarging it would only blur it.
fn fit_icon(bytes: &[u8], px: u32) -> Option<(image::Handle, Vec<u8>)> {
    let picture = ::image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = picture.dimensions();
    if width == 0 || height == 0 {
        return None;
    }
    let (fitted_width, fitted_height) = fit_within(width, height, px);
    let icon = if (fitted_width, fitted_height) == (width, height) {
        picture
    } else {
        imageops::resize(&picture, fitted_width, fitted_height, imageops::FilterType::Lanczos3)
    };
    let png = encode_png(&icon)?;
    Some((image::Handle::from_rgba(fitted_width, fitted_height, icon.into_raw()), png))
}

/// The size of a `width` by `height` picture fitted within a `side` square: its longer edge is `side`, its proportions are
/// kept, and a picture that already fits keeps its size.
fn fit_within(width: u32, height: u32, side: u32) -> (u32, u32) {
    let longest = width.max(height);
    if longest <= side {
        return (width, height);
    }
    let scale = f64::from(side) / f64::from(longest);
    (
        ((f64::from(width) * scale).round() as u32).max(1),
        ((f64::from(height) * scale).round() as u32).max(1),
    )
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
