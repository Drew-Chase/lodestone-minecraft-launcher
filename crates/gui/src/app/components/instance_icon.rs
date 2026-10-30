//! Instance icons: a background preset with a symbol, drawn as one rounded square, or an
//! uploaded image. The icon editor, the library cards and the instance page all draw
//! through here, so an icon looks the same wherever it appears.

use std::f32::consts::FRAC_PI_4;
use std::sync::OnceLock;

use iced::widget::svg::Handle as SvgHandle;
use iced::widget::{Space, container, image, svg};
use iced::{Background, Border, Color, ContentFit, Element, Fill, Gradient, alignment, gradient};

use crate::app::pages::library::instance::InstanceIcon;

/// A background preset: a gradient from its top-left corner to its bottom-right.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preset {
    /// The id the database stores.
    pub id: &'static str,
    start: Color,
    end: Color,
}

impl Preset {
    /// The gradient's first colour.
    pub fn start(self) -> Color {
        self.start
    }

    /// The gradient's last colour.
    pub fn end(self) -> Color {
        self.end
    }
}

/// The background presets, in the order the editor shows them.
pub const BACKGROUNDS: [Preset; 7] = [
    Preset {
        id: "rose",
        start: Color::from_rgb8(251, 113, 133),
        end: Color::from_rgb8(225, 29, 72),
    },
    Preset {
        id: "sunset",
        start: Color::from_rgb8(251, 146, 60),
        end: Color::from_rgb8(234, 88, 12),
    },
    Preset {
        id: "amber",
        start: Color::from_rgb8(253, 224, 71),
        end: Color::from_rgb8(234, 179, 8),
    },
    Preset {
        id: "lime",
        start: Color::from_rgb8(190, 242, 100),
        end: Color::from_rgb8(101, 163, 13),
    },
    Preset {
        id: "emerald",
        start: Color::from_rgb8(74, 222, 128),
        end: Color::from_rgb8(21, 128, 61),
    },
    Preset {
        id: "indigo",
        start: Color::from_rgb8(129, 140, 248),
        end: Color::from_rgb8(67, 56, 202),
    },
    Preset {
        id: "midnight",
        start: Color::from_rgb8(71, 85, 105),
        end: Color::from_rgb8(30, 41, 59),
    },
];

/// A symbol the icon can carry. Each one is an SVG drawn in a 64-unit square.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Symbol {
    /// The id the database stores: the file name without its extension.
    pub id: &'static str,
    svg: &'static str,
}

/// The symbols, in the order the editor shows them. Replace the files in
/// `res/artwork/symbols` to change the set.
pub const SYMBOLS: [Symbol; 16] = [
    Symbol {
        id: "grass",
        svg: include_str!("../../../../../res/artwork/symbols/grass.svg"),
    },
    Symbol {
        id: "stone",
        svg: include_str!("../../../../../res/artwork/symbols/stone.svg"),
    },
    Symbol {
        id: "dirt",
        svg: include_str!("../../../../../res/artwork/symbols/dirt.svg"),
    },
    Symbol {
        id: "gold",
        svg: include_str!("../../../../../res/artwork/symbols/gold.svg"),
    },
    Symbol {
        id: "diamond",
        svg: include_str!("../../../../../res/artwork/symbols/diamond.svg"),
    },
    Symbol {
        id: "emerald",
        svg: include_str!("../../../../../res/artwork/symbols/emerald.svg"),
    },
    Symbol {
        id: "amethyst",
        svg: include_str!("../../../../../res/artwork/symbols/amethyst.svg"),
    },
    Symbol {
        id: "redstone",
        svg: include_str!("../../../../../res/artwork/symbols/redstone.svg"),
    },
    Symbol {
        id: "sand",
        svg: include_str!("../../../../../res/artwork/symbols/sand.svg"),
    },
    Symbol {
        id: "clay",
        svg: include_str!("../../../../../res/artwork/symbols/clay.svg"),
    },
    Symbol {
        id: "obsidian",
        svg: include_str!("../../../../../res/artwork/symbols/obsidian.svg"),
    },
    Symbol {
        id: "lapis",
        svg: include_str!("../../../../../res/artwork/symbols/lapis.svg"),
    },
    Symbol {
        id: "quartz",
        svg: include_str!("../../../../../res/artwork/symbols/quartz.svg"),
    },
    Symbol {
        id: "copper",
        svg: include_str!("../../../../../res/artwork/symbols/copper.svg"),
    },
    Symbol {
        id: "moss",
        svg: include_str!("../../../../../res/artwork/symbols/moss.svg"),
    },
    Symbol {
        id: "ice",
        svg: include_str!("../../../../../res/artwork/symbols/ice.svg"),
    },
];

/// The handles for [`SYMBOLS`], built once so redraws reuse their rasterisation.
static SYMBOL_HANDLES: OnceLock<Vec<SvgHandle>> = OnceLock::new();

/// The preset with `id`, or the first one when the id is unknown.
pub fn preset(id: &str) -> Preset {
    BACKGROUNDS.iter().copied().find(|preset| preset.id == id).unwrap_or(BACKGROUNDS[0])
}

/// The symbol with `id`, or the first one when the id is unknown.
pub fn symbol(id: &str) -> Symbol {
    SYMBOLS.iter().copied().find(|symbol| symbol.id == id).unwrap_or(SYMBOLS[0])
}

/// A random preset and symbol, for new instances and the Randomize button.
pub fn random() -> (&'static str, &'static str) {
    let preset = BACKGROUNDS[fastrand::usize(..BACKGROUNDS.len())];
    let symbol = SYMBOLS[fastrand::usize(..SYMBOLS.len())];
    (preset.id, symbol.id)
}

/// The icon as it is drawn at `size` pixels square.
pub fn icon<'a, M: 'a>(icon: &InstanceIcon, size: f32) -> Element<'a, M> {
    match icon {
        InstanceIcon::Composed { background, symbol: name } => composed(background, name, size),
        InstanceIcon::Custom(path) => container(
            image(image::Handle::from_path(path))
                .width(Fill)
                .height(Fill)
                .content_fit(ContentFit::Cover),
        )
        .width(size)
        .height(size)
        .clip(true)
        .style(move |_| rounded(size))
        .into(),
    }
}

/// A composed icon: the preset's gradient with the symbol centred on it.
pub fn composed<'a, M: 'a>(background: &str, symbol_id: &str, size: f32) -> Element<'a, M> {
    let preset = preset(background);
    let glyph = size * 0.62;
    let mark = svg(symbol_handle(symbol_id)).width(glyph).height(glyph);
    container(mark)
        .width(size)
        .height(size)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .clip(true)
        .style(move |_| gradient_style(preset, size))
        .into()
}

/// The preset's gradient inside the rounded-square chrome.
fn gradient_style(preset: Preset, size: f32) -> container::Style {
    container::Style {
        background: Some(Background::Gradient(Gradient::Linear(
            gradient::Linear::new(3.0 * FRAC_PI_4)
                .add_stop(0.0, preset.start())
                .add_stop(1.0, preset.end()),
        ))),
        ..rounded(size)
    }
}

/// An uploaded image, drawn as the icon would be: square, rounded, cover-cropped.
pub fn bytes<'a, M: 'a>(handle: image::Handle, size: f32) -> Element<'a, M> {
    container(image(handle).width(Fill).height(Fill).content_fit(ContentFit::Cover))
        .width(size)
        .height(size)
        .clip(true)
        .style(move |_| rounded(size))
        .into()
}

/// A background preset's gradient alone, for the editor's swatches.
pub fn swatch<'a, M: 'a>(background: &str, size: f32) -> Element<'a, M> {
    let preset = preset(background);
    container(Space::new().width(size).height(size))
        .width(size)
        .height(size)
        .clip(true)
        .style(move |_| gradient_style(preset, size))
        .into()
}

/// The rounded-square chrome an icon of `size` pixels wears.
fn rounded(size: f32) -> container::Style {
    container::Style {
        border: Border {
            radius: (size * 0.24).into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// The cached handle for the symbol with `id`.
fn symbol_handle(id: &str) -> SvgHandle {
    let handles = SYMBOL_HANDLES.get_or_init(|| SYMBOLS.iter().map(|symbol| SvgHandle::from_memory(symbol.svg.as_bytes())).collect());
    let index = SYMBOLS.iter().position(|symbol| symbol.id == id).unwrap_or(0);
    handles[index].clone()
}

#[cfg(test)]
#[path = "../../../tests/app/components/instance_icon.rs"]
mod tests;
