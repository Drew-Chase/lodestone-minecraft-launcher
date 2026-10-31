//! An instance's banner: the uploaded image when it has one, otherwise the scene art. The library's cards,
//! thumbnails and hero draw the scene picture the art cache rendered for them, and show a dark panel until
//! it lands. The instance masthead picks through here too.

use std::f32::consts::FRAC_PI_2;
use std::path::Path;

use iced::border::Radius;
use iced::widget::image::Handle;
use iced::widget::{Space, container, image, stack};
use iced::{Background, Border, Color, ContentFit, Element, Fill, Gradient, gradient};
use iced_color_extension::rgb;

use crate::app::components::scene_art::{CARD_RADIUS, HERO_RADIUS};
use crate::app::pages::library::instance::Instance;

/// The uploaded banner, covering its box. Callers clip it to their own corners.
pub fn custom_banner<'a, M: 'a>(path: &Path) -> Element<'a, M> {
    image(image::Handle::from_path(path))
        .width(Fill)
        .height(Fill)
        .content_fit(ContentFit::Cover)
        .into()
}

/// The banner a grid card shows: the uploaded image, or the scene picture `rendered` at the box's size,
/// which already has its top corners rounded. Until the picture renders, a dark panel shows.
pub fn card_banner<'a, M: 'a>(instance: &Instance, rendered: Option<Handle>) -> Element<'a, M> {
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
        None => picture(rendered, Radius::default()),
    }
}

/// The square thumbnail a compact row shows: the uploaded image, or the scene picture, covering the box.
/// Its corners stay square, as the row's art always has.
pub fn thumbnail<'a, M: 'a>(instance: &Instance, rendered: Option<Handle>) -> Element<'a, M> {
    match &instance.art_path {
        Some(path) => custom_banner(path),
        None => picture(rendered, Radius::default()),
    }
}

/// The library hero's banner, rounded to its frame. The scene picture has its rounding and left fade
/// baked in, so an uploaded image gets its rounding from the image widget and its fade from a rounded layer.
pub fn hero_banner<'a, M: 'a>(instance: &Instance, rendered: Option<Handle>) -> Element<'a, M> {
    match &instance.art_path {
        Some(path) => stack![
            image(image::Handle::from_path(path))
                .width(Fill)
                .height(Fill)
                .content_fit(ContentFit::Cover)
                .border_radius(Radius::from(HERO_RADIUS)),
            readability_fade(),
        ]
        .width(Fill)
        .height(Fill)
        .into(),
        None => picture(rendered, Radius::from(HERO_RADIUS)),
    }
}

/// A rendered scene picture filling its box, or the dark panel it shows while it renders. The panel takes
/// `corners`, so a hero's panel rounds like its picture.
fn picture<'a, M: 'a>(rendered: Option<Handle>, corners: Radius) -> Element<'a, M> {
    match rendered {
        Some(handle) => image(handle).width(Fill).height(Fill).content_fit(ContentFit::Cover).into(),
        None => container(Space::new().width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .style(move |_| container::Style {
                background: Some(Background::Color(Color::from_rgb8(0x0D, 0x0F, 0x12))),
                border: Border {
                    radius: corners,
                    ..Border::default()
                },
                ..container::Style::default()
            })
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
            border: Border {
                radius: HERO_RADIUS.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
}
