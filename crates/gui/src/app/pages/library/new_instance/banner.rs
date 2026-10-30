//! The banner artwork: the scene choices, their thumbnails, and the frames the banner shows in.
//! The form's banner strip and the banner editor's grid both draw from here.

use iced::widget::{container, image, svg};
use iced::{Background, Border, ContentFit, Element, Fill, Length, Padding};
use iced_color_extension::rgb;

use crate::app::components::scene_art::scene_handle;
use crate::app::components::tile::tile;
use crate::app::pages::library::instance::Biome;

/// The scene slots in artwork order. Each slot has a base scene and two variants.
const BIOMES: [Biome; 8] = [
    Biome::End,
    Biome::Forest,
    Biome::Cherry,
    Biome::Ocean,
    Biome::Desert,
    Biome::Snow,
    Biome::Nether,
    Biome::Mushroom,
];

/// The scenes a banner can pick: 8 biomes times the base scene and its two variants.
pub const SCENE_COUNT: usize = 24;

/// The biome and art seed of a scene choice. The seed picks the base scene or one of its
/// two variants, which is how `scene_art` resolves a pair.
pub fn scene(index: usize) -> (Biome, u64) {
    (BIOMES[(index / 3) % BIOMES.len()], (index % 3) as u64)
}

/// A random scene choice, for new instances.
pub fn random_scene() -> usize {
    fastrand::usize(..SCENE_COUNT)
}

/// The name of a scene choice, such as "Forest, variant 2", for the editor's caption.
pub fn scene_name(index: usize) -> String {
    let (biome, seed) = scene(index);
    let name = match biome {
        Biome::End => "End",
        Biome::Forest => "Forest",
        Biome::Cherry => "Cherry",
        Biome::Ocean => "Ocean",
        Biome::Desert => "Desert",
        Biome::Snow => "Snow",
        Biome::Nether => "Nether",
        Biome::Mushroom => "Mushroom",
    };
    if seed == 0 {
        format!("{name}, base scene")
    } else {
        format!("{name}, variant {seed}")
    }
}

/// The scene artwork, covering the box it is placed in.
pub fn art<'a, M: 'a>(index: usize) -> Element<'a, M> {
    let (biome, seed) = scene(index);
    svg(scene_handle(biome, seed))
        .content_fit(ContentFit::Cover)
        .width(Fill)
        .height(Fill)
        .into()
}

/// The thumbnail of scene `index`, `width` pixels wide in its 16:9 frame. It rings when
/// `selected`, lifts and outlines under the pointer, and sends `on_press` when clicked.
pub fn thumbnail<'a, M: Clone + 'a>(index: usize, selected: bool, width: f32, on_press: M) -> Element<'a, M> {
    let height = width * 9.0 / 16.0;
    tile(clipped(art(index), Fill, Fill, 9.0), width, height, 10.0, selected, on_press)
}

/// An uploaded banner, covering the box it is placed in.
pub fn uploaded<'a, M: 'a>(handle: image::Handle) -> Element<'a, M> {
    image(handle).width(Fill).height(Fill).content_fit(ContentFit::Cover).into()
}

/// `content` in a box of the given size, clipped to rounded corners.
pub fn clipped<'a, M: 'a>(content: impl Into<Element<'a, M>>, width: impl Into<Length>, height: impl Into<Length>, radius: f32) -> Element<'a, M> {
    container(content)
        .width(width)
        .height(height)
        .clip(true)
        .style(move |_| container::Style {
            border: Border {
                radius: radius.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}

/// The padding of a label pill that sits over a banner.
pub fn pill_padding() -> Padding {
    Padding::new(0.0).left(10.0).right(10.0).top(4.0).bottom(4.0)
}

/// The dark pill a label sits on over a banner.
pub fn pill_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(rgb!(0, 0, 0, 60%))),
        border: Border {
            radius: 999.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/library/new_instance/banner.rs"]
mod tests;
