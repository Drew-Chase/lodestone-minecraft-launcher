//! The masthead: the generated biome band with its chrome buttons, the
//! identity block and its badges, the session controls, and the instance icon
//! that hangs below the band.

use aurora_leaf::Icon;
use chrono::Local;
use iced::widget::{Space, Text, column, container, responsive, row, stack, svg};
use iced::{Background, Border, Color, ContentFit, Element, Fill, Gradient, Padding, Shadow, Size, alignment, gradient};
use iced_color_extension::rgb;

use crate::app::components::badge::{Badge, loader_palette};
use crate::app::components::button::Variant;
use crate::app::components::icon_button::IconButtonStyle;
use crate::app::components::scene_art::banner_handle;
use crate::app::components::{IconButton, TextButton};
use crate::app::components::{instance_art, instance_icon};
use crate::app::lang;
use crate::app::pages::library::instance::{Instance, InstanceState};
use crate::app::theme::{color, fonts, utility};

use super::files::InstanceFiles;
use super::format::format_bytes;
use super::worlds::format_saved;
use super::{GUTTER, Message, layer};

/// The masthead's height — the design's 230px scene band.
const HERO_HEIGHT: f32 = 230.0;
/// The instance icon's edge.
const ICON_SIZE: f32 = 110.0;
/// The icon hangs 40px below the masthead (`translateY(40px)`).
const ICON_TOP: f32 = HERO_HEIGHT - ICON_SIZE + 40.0;
/// Left edge of the identity text — icon + the design's 20px gap.
const IDENTITY_LEFT: f32 = GUTTER + ICON_SIZE + 20.0;
/// Below this width the session controls move under the identity block.
const NARROW_WIDTH: f32 = 1020.0;

/// The masthead's toggles, owned by the page.
#[derive(Debug, Clone, Copy, Default)]
pub struct Controls {
    pub favorite: bool,
    pub paused: bool,
}

/// The masthead for `instance`, laid out for a window of `viewport` size. `files`
/// is what the folder held at the last scan; until one lands, its figures show a dash.
pub fn hero<'a>(instance: &'a Instance, files: Option<&InstanceFiles>, controls: Controls, viewport: Size) -> Element<'a, Message> {
    // The art is generated at the masthead's exact pixel size, so it fills
    // the band without stretching.
    let (biome, seed) = (instance.biome, instance.art_seed);
    let upload = instance.art_path.clone();
    let art = responsive(move |size| match &upload {
        Some(path) => instance_art::custom_banner(path),
        None => svg(banner_handle(biome, seed, size))
            .content_fit(ContentFit::Fill)
            .width(Fill)
            .height(Fill)
            .into(),
    })
    .height(HERO_HEIGHT);

    // `linear-gradient(180deg, rgba(8,9,10,.3), rgba(8,9,10,.85) 85%, bg)`
    let fade = container(Space::new().width(Fill).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(|_| container::Style {
            background: Some(Background::Gradient(Gradient::Linear(
                gradient::Linear::new(std::f32::consts::PI)
                    .add_stop(0.0, rgb!(8, 9, 10, 30%))
                    .add_stop(0.85, rgb!(8, 9, 10, 85%))
                    .add_stop(1.0, color::Surface::APPLICATION),
            ))),
            ..container::Style::default()
        });

    let back = layer(
        glass_button(Icon::lucide().chevron_left(), Message::Back),
        alignment::Horizontal::Left,
        alignment::Vertical::Top,
        Padding::new(0.0).left(24.0).top(16.0),
    );

    let favorite = if controls.favorite {
        glass_button_with_style(Icon::lucide().heart(), favorite_style(), Message::ToggleFavorite)
    } else {
        glass_button(Icon::lucide().heart(), Message::ToggleFavorite)
    };
    let top_right = layer(
        row![
            favorite,
            glass_button(Icon::lucide().external_link(), Message::RevealFolder),
            glass_button(Icon::lucide().ellipsis_vertical(), Message::More),
        ]
        .spacing(8)
        .into(),
        alignment::Horizontal::Right,
        alignment::Vertical::Top,
        Padding::new(0.0).right(24.0).top(16.0),
    );

    let identity = column![
        row![
            status_badge(instance.state),
            loader_badge(instance),
            Badge::new(format!("MC {}", instance.mc_version), color::Text::MUTED, rgb!(255, 255, 255, 6%)),
            Badge::new(mods_label(files), color::Accents::AMBER, color::Accents::AMBER_TINT),
        ]
        .spacing(6),
        Text::new(instance.name.clone())
            .size(32)
            .line_height(1.0)
            .font(fonts::Inter::EXTRA_BOLD)
            .color(color::Text::HEADLINE),
        Text::new(meta_line(instance, files))
            .size(12)
            .line_height(1.2)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::PLACEHOLDER),
    ]
    .spacing(7);

    let playing = instance.state == InstanceState::Playing;
    let narrow = viewport.width < NARROW_WIDTH;
    let identity = if narrow {
        identity.push(session_controls(playing, controls.paused)).spacing(12)
    } else {
        identity
    };
    let actions_layer = (!narrow).then(|| {
        layer(
            session_controls(playing, controls.paused),
            alignment::Horizontal::Right,
            alignment::Vertical::Bottom,
            Padding::new(0.0).right(GUTTER).bottom(18.0),
        )
    });
    let identity_layer = layer(
        identity.into(),
        alignment::Horizontal::Left,
        alignment::Vertical::Bottom,
        Padding::new(0.0).left(IDENTITY_LEFT).bottom(18.0),
    );

    let mut surface = stack![art, fade, back, top_right, identity_layer];
    if let Some(actions) = actions_layer {
        surface = surface.push(actions);
    }

    container(surface.width(Fill).height(HERO_HEIGHT)).width(Fill).height(HERO_HEIGHT).into()
}

/// The instance icon: 110px square, hanging 40px below the masthead. The page
/// shell layers it over the whole page, since inside the masthead it would be
/// clamped to the 70px beneath its top offset.
pub fn hero_icon(instance: &Instance) -> Element<'static, Message> {
    let icon = container(instance_icon::icon(&instance.icon, ICON_SIZE))
        .width(ICON_SIZE)
        .height(ICON_SIZE)
        .style(|_| container::Style {
            background: Some(Background::Color(color::Surface::APPLICATION)),
            border: Border {
                radius: 18.0.into(),
                width: 3.0,
                color: color::Surface::APPLICATION,
            },
            shadow: Shadow {
                color: rgb!(0, 0, 0, 80%),
                offset: iced::Vector::new(0.0, 20.0),
                blur_radius: 40.0,
            },
            ..container::Style::default()
        });

    layer(
        icon.into(),
        alignment::Horizontal::Left,
        alignment::Vertical::Top,
        Padding::new(0.0).left(GUTTER).top(ICON_TOP),
    )
}

/// Pause plus the running-session button.
fn session_controls(playing: bool, paused: bool) -> Element<'static, Message> {
    let pause_label = if paused {
        lang::t("instance.hero.resume")
    } else {
        lang::t("instance.hero.pause")
    };

    row![
        TextButton::content(
            row![
                utility::icon(Icon::lucide().pause(), 13.0, Some((color::Text::DEFAULT, color::Text::DEFAULT))),
                Text::new(pause_label).size(13).font(fonts::Inter::MEDIUM).color(color::Text::DEFAULT),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        )
        .variant(Variant::Ghost)
        .padding(Padding::new(0.0).left(16).right(16).top(10).bottom(10))
        .on_press(Message::TogglePause),
        TextButton::content(
            row![
                utility::icon(Icon::lucide().play(), 12.0, Some((color::Primary::ON, color::Primary::ON))),
                Text::new(if playing {
                    lang::t("instance.hero.running")
                } else {
                    lang::t("instance.hero.launch")
                })
                .size(14)
                .font(fonts::Inter::BOLD)
                .color(color::Primary::ON),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        )
        .variant(Variant::Primary)
        .padding(Padding::new(0.0).left(22).right(22).top(11).bottom(11))
        .on_press(Message::TogglePause),
    ]
    .spacing(8)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// A 36px glass chrome button over the masthead art.
fn glass_button(markup: &'static str, message: Message) -> Element<'static, Message> {
    glass_button_with_style(markup, glass_style(), message)
}

fn glass_button_with_style(markup: &'static str, style: IconButtonStyle, message: Message) -> Element<'static, Message> {
    container(
        IconButton::new(markup)
            .size(36)
            .radius(12.0)
            .icon_size(14.0)
            .style(style)
            .on_press(message),
    )
    .style(|_| container::Style {
        background: Some(Background::Color(rgb!(0, 0, 0, 40%))),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: color::Surface::LINE,
        },
        ..container::Style::default()
    })
    .into()
}

fn glass_style() -> IconButtonStyle {
    IconButtonStyle {
        foreground: color::Text::MUTED,
        foreground_hover: color::Text::HEADLINE,
        background: Color::TRANSPARENT,
        border: Color::TRANSPARENT,
        background_hover: rgb!(255, 255, 255, 8%),
        border_hover: Color::TRANSPARENT,
        background_disabled: Color::TRANSPARENT,
        foreground_disabled: color::Text::DISABLED,
        background_pressed: rgb!(255, 255, 255, 12%),
        foreground_pressed: color::Text::HEADLINE,
    }
}

fn favorite_style() -> IconButtonStyle {
    IconButtonStyle {
        foreground: color::Accents::PINK,
        foreground_hover: color::Accents::PINK,
        background: Color::TRANSPARENT,
        background_hover: color::Accents::PINK_TINT,
        ..glass_style()
    }
}

/// The launched/idle badge.
fn status_badge(state: InstanceState) -> Element<'static, Message> {
    let (label, ink, fill, pulsing) = match state {
        InstanceState::Playing => (lang::t("instance.status.launched"), color::Primary::DEFAULT, color::Primary::TINT, true),
        InstanceState::Idle => (lang::t("instance.status.idle"), color::Text::MUTED, rgb!(255, 255, 255, 6%), false),
    };
    Badge::new(label, ink, fill).with_pulse(pulsing).into()
}

/// The loader badge, with the loader's version when it has one.
fn loader_badge(instance: &Instance) -> Element<'static, Message> {
    let (ink, fill) = loader_palette(instance.loader);
    let label = match &instance.loader_version {
        Some(version) => format!("{} {version}", instance.loader.label()),
        None => instance.loader.label().to_string(),
    };
    Badge::new(label, ink, fill).with_loader(instance.loader).into()
}

/// The mono identity line: folder, size on disk, and when a world was last saved.
fn meta_line(instance: &Instance, files: Option<&InstanceFiles>) -> String {
    let Some(files) = files else {
        return format!("/instances/{} · —", instance.directory_name);
    };
    let saved = files
        .worlds
        .iter()
        .filter_map(|world| world.saved)
        .max()
        .map_or_else(|| lang::t("instance.hero.no_saves").to_owned(), |saved| format_saved(saved, Local::now()));
    format!("/instances/{} · {} · {saved}", instance.directory_name, format_bytes(files.folder_bytes))
}

/// The mod count for the masthead badge, or a dash until the folder is read.
fn mods_label(files: Option<&InstanceFiles>) -> String {
    files.map_or_else(
        || lang::t("instance.hero.mods_unknown").to_owned(),
        |files| lang::t_count("instance.mods_caps", files.mods.len()),
    )
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/instance/hero.rs"]
mod tests;
