//! Mod loader marks: each loader's logo drawn as one square. The New Instance cards and the
//! loader badges both draw through here, so a loader looks the same wherever it appears.

use std::sync::LazyLock;

use aurora_leaf::Icon;
use iced::Element;
use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{image, svg};

use crate::app::components::badge::loader_palette;
use crate::app::components::instance_icon;
use crate::app::pages::library::instance::Loader;

/// The NeoForge mark, embedded from `res/artwork/loaders`.
const NEOFORGE_PNG: &[u8] = include_bytes!("../../../../../res/artwork/loaders/neoforge.png");
/// The Quilt mark, embedded from `res/artwork/loaders`.
const QUILT_PNG: &[u8] = include_bytes!("../../../../../res/artwork/loaders/quilt.png");

// Built once, so redraws reuse each handle's rasterisation. An `image::Handle` gets a new id on
// every `from_bytes`, so a fresh one per redraw would decode its mark again each frame.
static VANILLA: LazyLock<SvgHandle> = LazyLock::new(|| SvgHandle::from_memory(Icon::selfhst().minecraft().as_bytes()));
// The Fabric cube is drawn from (1, 1) to (14, 15) on its 16 unit grid, so that box is the frame it fills.
static FABRIC: LazyLock<SvgHandle> =
    LazyLock::new(|| SvgHandle::from_memory(reframe(Icon::material_icon_theme().minecraft_fabric(), (1, 1, 13, 14))));
static FORGE: LazyLock<SvgHandle> = LazyLock::new(|| SvgHandle::from_memory(reframe(Icon::at_icons().anvil(), (0, 0, 16, 16))));
static NEOFORGE: LazyLock<image::Handle> = LazyLock::new(|| image::Handle::from_bytes(NEOFORGE_PNG));
static QUILT: LazyLock<image::Handle> = LazyLock::new(|| image::Handle::from_bytes(QUILT_PNG));

/// Reframes an aurora_iconify mark onto the box its glyph fills, in the mark's own units. Iconify's
/// default icon size is 16, but aurora_iconify boxes an icon with no size of its own in 24 units,
/// which left the glyph small in the top-left corner. Once the default is read correctly, the 24
/// unit box is gone and the markup passes through unchanged.
fn reframe(markup: &str, (x, y, width, height): (u32, u32, u32, u32)) -> Vec<u8> {
    let frame = format!(r#"width="{width}" height="{height}" viewBox="{x} {y} {width} {height}""#);
    markup.replacen(r#"width="24" height="24" viewBox="0 0 24 24""#, &frame, 1).into_bytes()
}

/// The loader's mark as a `size` pixel square. The Forge anvil is monochrome, so it takes the
/// loader's ink. The other marks keep the colours they were drawn in.
pub fn loader_icon<'a, M: 'a>(loader: Loader, size: f32) -> Element<'a, M> {
    match loader {
        Loader::Vanilla => svg((*VANILLA).clone()).width(size).height(size).into(),
        Loader::Fabric => svg((*FABRIC).clone()).width(size).height(size).into(),
        Loader::Forge => {
            let (ink, _) = loader_palette(loader);
            svg((*FORGE).clone())
                .width(size)
                .height(size)
                .style(move |_, _| svg::Style { color: Some(ink) })
                .into()
        }
        Loader::NeoForge => instance_icon::bytes((*NEOFORGE).clone(), size),
        Loader::Quilt => instance_icon::bytes((*QUILT).clone(), size),
    }
}

#[cfg(test)]
#[path = "../../../tests/app/components/loader_icon.rs"]
mod tests;
