//! An instance's banner: the uploaded image when it has one, otherwise the generated scene
//! art. The library cards, the library hero and the instance masthead all pick through
//! here.

use std::f32::consts::FRAC_PI_2;
use std::path::Path;

use iced::border::Radius;
use iced::widget::{Space, container, image, responsive, stack, svg};
use iced::{Background, Border, ContentFit, Element, Fill, Gradient, gradient};
use iced_color_extension::rgb;

use crate::app::components::scene_art::{CARD_RADIUS, card_handle, hero_handle, scene_handle};
use crate::app::pages::library::instance::Instance;

/// The uploaded banner, covering its box. Callers clip it to their own corners.
pub fn custom_banner<'a, M: 'a>(path: &Path) -> Element<'a, M> {
    image(image::Handle::from_path(path))
        .width(Fill)
        .height(Fill)
        .content_fit(ContentFit::Cover)
        .into()
}

/// The banner a grid card shows: the uploaded image, or the scene art generated at the box's exact
/// size. Both are rounded at the top to the card's border. A container cannot clip them that way,
/// since its clip is rectangular, so the rounding is part of the image itself.
pub fn card_banner<'a, M: 'a>(instance: &Instance) -> Element<'a, M> {
    match &instance.art_path {
        Some(path) => image(image::Handle::from_path(path))
            .width(Fill)
            .height(Fill)
            .content_fit(ContentFit::Cover)
            .border_radius(Radius {
                top_left: CARD_RADIUS,
                top_right: CARD_RADIUS,
                bottom_right: 0.0,
                bottom_left: 0.0,
            })
            .into(),
        None => {
            let (biome, seed) = (instance.biome, instance.art_seed);
            responsive(move |size| {
                svg(card_handle(biome, seed, size))
                    .content_fit(ContentFit::Cover)
                    .width(Fill)
                    .height(Fill)
                    .into()
            })
            .into()
        }
    }
}

/// The square thumbnail the Compact row shows: the uploaded image, or the scene art, covering the
/// box. Its corners stay square, as the row's art always has.
pub fn thumbnail<'a, M: 'a>(instance: &Instance) -> Element<'a, M> {
    match &instance.art_path {
        Some(path) => custom_banner(path),
        None => svg(scene_handle(instance.biome, instance.art_seed))
            .content_fit(ContentFit::Cover)
            .width(Fill)
            .height(Fill)
            .into(),
    }
}

/// The library hero's banner, rounded to its frame. The scene art has its rounding and
/// left fade baked in, so an uploaded image gets both here instead.
pub fn hero_banner<'a, M: 'a>(instance: &Instance) -> Element<'a, M> {
    match &instance.art_path {
        Some(path) => container(stack![custom_banner(path), readability_fade()])
            .width(Fill)
            .height(Fill)
            .clip(true)
            .style(|_| container::Style {
                border: Border {
                    radius: 18.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            })
            .into(),
        None => svg(hero_handle(instance.biome, instance.art_seed))
            .content_fit(ContentFit::Cover)
            .width(Fill)
            .height(Fill)
            .into(),
    }
}

/// The left-dark fade the hero variants bake in, over an uploaded image: the same stops as
/// the scene art's hero wrapper (`hero_wrapped`).
fn readability_fade<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new().width(Fill).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(|_| container::Style {
            background: Some(Background::Gradient(Gradient::Linear(
                gradient::Linear::new(FRAC_PI_2)
                    .add_stop(0.0, rgb!(6, 8, 10, 92%))
                    .add_stop(0.4, rgb!(6, 8, 10, 49%))
                    .add_stop(0.65, rgb!(6, 8, 10, 0%)),
            ))),
            ..container::Style::default()
        })
        .into()
}
