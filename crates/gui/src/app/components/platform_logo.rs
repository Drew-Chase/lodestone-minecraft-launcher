//! The marks of the five content platforms. Modrinth and CurseForge are
//! monochrome icons from Iconify's Simple Icons set (CC0), drawn in each
//! platform's brand colour. ATLauncher and FTB ship their own colours. Technic
//! has no vector mark on its site, so its favicon stands in as a raster.

use std::sync::OnceLock;

use iced::widget::{Space, image, svg};
use iced::{Element, Length};

/// One platform's mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Logo {
    Modrinth,
    CurseForge,
    AtLauncher,
    Technic,
    Ftb,
}

const MODRINTH: &str = include_str!("../../../../../res/artwork/platforms/modrinth.svg");
const CURSEFORGE: &str = include_str!("../../../../../res/artwork/platforms/curseforge.svg");
const ATLAUNCHER: &str = include_str!("../../../../../res/artwork/platforms/atlauncher.svg");
const FTB: &str = include_str!("../../../../../res/artwork/platforms/ftb.svg");
const TECHNIC: &[u8] = include_bytes!("../../../../../res/artwork/platforms/technic.ico");

/// Modrinth's brand green.
const MODRINTH_INK: &str = "#1bd96a";
/// CurseForge's brand orange.
const CURSEFORGE_INK: &str = "#f16436";

static MODRINTH_HANDLE: OnceLock<svg::Handle> = OnceLock::new();
static CURSEFORGE_HANDLE: OnceLock<svg::Handle> = OnceLock::new();
static ATLAUNCHER_HANDLE: OnceLock<svg::Handle> = OnceLock::new();
static FTB_HANDLE: OnceLock<svg::Handle> = OnceLock::new();
static TECHNIC_HANDLE: OnceLock<Option<image::Handle>> = OnceLock::new();

/// The mark at `size` pixels square.
pub fn logo<'a, Message: 'a>(logo: Logo, size: f32) -> Element<'a, Message> {
    match logo {
        Logo::Modrinth => svg(vector(&MODRINTH_HANDLE, MODRINTH, Some(MODRINTH_INK)))
            .width(size)
            .height(size)
            .into(),
        Logo::CurseForge => svg(vector(&CURSEFORGE_HANDLE, CURSEFORGE, Some(CURSEFORGE_INK)))
            .width(size)
            .height(size)
            .into(),
        Logo::AtLauncher => svg(vector(&ATLAUNCHER_HANDLE, ATLAUNCHER, None)).width(size).height(size).into(),
        Logo::Ftb => svg(vector(&FTB_HANDLE, FTB, None)).width(size).height(size).into(),
        Logo::Technic => match technic_handle() {
            Some(handle) => image(handle).width(size).height(size).into(),
            None => Space::new().width(Length::Fixed(size)).height(Length::Fixed(size)).into(),
        },
    }
}

/// The SVG handle for `source`, built once. A monochrome source takes `ink` in
/// place of its `currentColor` fills.
fn vector(cell: &'static OnceLock<svg::Handle>, source: &'static str, ink: Option<&'static str>) -> svg::Handle {
    cell.get_or_init(|| {
        let markup = match ink {
            Some(color) => source.replace("currentColor", color),
            None => source.to_string(),
        };
        svg::Handle::from_memory(markup.into_bytes())
    })
    .clone()
}

/// The favicon's raster, decoded once. `None` when the bytes do not decode,
/// which leaves the slot empty rather than failing the page.
fn technic_handle() -> Option<image::Handle> {
    TECHNIC_HANDLE.get_or_init(decode_technic).clone()
}

fn decode_technic() -> Option<image::Handle> {
    let decoded = ::image::load_from_memory_with_format(TECHNIC, ::image::ImageFormat::Ico).ok()?.to_rgba8();
    let (width, height) = decoded.dimensions();
    Some(image::Handle::from_rgba(width, height, decoded.into_raw()))
}
