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
use crate::app::components::hint::hint;
use crate::app::components::icon_button::IconButtonStyle;
use crate::app::components::scene_art::banner_handle;
use crate::app::components::{Dropdown, IconButton, TextButton};
use crate::app::components::{instance_art, instance_icon};
use crate::app::lang;
use crate::app::launch::games::LaunchStatus;
use crate::app::pages::library::instance::Instance;
use crate::app::pages::library::instance_menu;
use crate::app::theme::{color, fonts, utility};

use super::files::InstanceFiles;
use super::format::format_bytes;
use super::worlds::format_saved;
use super::{GUTTER, Message, Tab, layer};

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

/// The masthead's controls: the state of the instance's game, and, while the instance installs, the line
/// the install shows in place of Launch.
#[derive(Debug, Clone, Default)]
pub struct Controls {
    pub status: LaunchStatus,
    pub install: Option<String>,
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

    // An instance from an external source links to its page; a local one reveals its folder.
    let source = match &instance.source_url {
        Some(_) => hint(
            glass_button(Icon::lucide().external_link(), Message::OpenSource),
            lang::t("instance.hero.open_source"),
        ),
        None => hint(
            glass_button(Icon::lucide().folder(), Message::RevealFolder),
            lang::t("instance.menu.reveal"),
        ),
    };
    let top_right = layer(
        row![source, glass_menu(instance, controls.install.is_some())].spacing(8).into(),
        alignment::Horizontal::Right,
        alignment::Vertical::Top,
        Padding::new(0.0).right(24.0).top(16.0),
    );

    let identity = column![
        row![
            status_badge(controls.status),
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

    let narrow = viewport.width < NARROW_WIDTH;
    let identity = if narrow {
        identity
            .push(session_controls(controls.status, &instance.id, controls.install.clone()))
            .spacing(12)
    } else {
        identity
    };
    let actions_layer = (!narrow).then(|| {
        layer(
            session_controls(controls.status, &instance.id, controls.install.clone()),
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

/// The session controls. While the game is stopped, Launch starts it. While it is being prepared,
/// Launch shows how far the files have got and is disabled. While it runs: a button that starts another
/// process of the instance, Restart and Console as icon buttons, and Kill at the far right.
fn session_controls(status: LaunchStatus, instance: &str, install: Option<String>) -> Element<'static, Message> {
    if let Some(label) = install {
        return row![launch_button(label, None)].spacing(8).align_y(alignment::Vertical::Center).into();
    }
    match status {
        LaunchStatus::Idle => row![launch_button(lang::t("instance.hero.launch").to_owned(), Some(Message::Launch))]
            .spacing(8)
            .align_y(alignment::Vertical::Center)
            .into(),
        LaunchStatus::Preparing(fraction) => {
            let label = match fraction {
                Some(fraction) => lang::t_with(
                    "instance.hero.preparing_percent",
                    &[("percent", format!("{:.0}", fraction * 100.0).as_str())],
                ),
                None => lang::t("instance.hero.preparing").to_owned(),
            };
            row![launch_button(label, None)].spacing(8).align_y(alignment::Vertical::Center).into()
        }
        LaunchStatus::Running => row![
            hint(new_process_button(instance), lang::t("game.new_process")),
            hint(restart_button(), lang::t("instance.hero.restart")),
            hint(console_button(), lang::t("instance.hero.console")),
            kill_button(),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center)
        .into(),
    }
}

/// Restart: an icon-only button in warning amber.
fn restart_button() -> Element<'static, Message> {
    IconButton::new(Icon::lucide().rotate_cw())
        .size(36)
        .radius(12.0)
        .icon_size(14.0)
        .style(restart_style())
        .on_press(Message::Restart)
        .into()
}

/// Starts another process of the instance: an icon-only ghost button, beside Console.
fn new_process_button(instance: &str) -> Element<'static, Message> {
    IconButton::new(Icon::lucide().copy())
        .size(36)
        .radius(12.0)
        .icon_size(14.0)
        .style(ghost_style())
        .on_press(Message::LaunchNew(instance.to_owned()))
        .into()
}

/// Console: an icon-only ghost button with no background and no border.
fn console_button() -> Element<'static, Message> {
    IconButton::new(Icon::lucide().terminal())
        .size(36)
        .radius(12.0)
        .icon_size(14.0)
        .style(ghost_style())
        .on_press(Message::TabSelected(Tab::Logs))
        .into()
}

/// Kill: the solid red button that ends the game, labelled, at the far right of the row.
fn kill_button() -> Element<'static, Message> {
    TextButton::content(
        row![
            utility::icon(Icon::lucide().square(), 11.0, Some((color::Text::HEADLINE, color::Text::HEADLINE))),
            Text::new(lang::t("instance.hero.kill"))
                .size(13)
                .font(fonts::Inter::BOLD)
                .color(color::Text::HEADLINE),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .variant(Variant::Destructive)
    .padding(Padding::new(0.0).left(20).right(20).top(10).bottom(10))
    .on_press(Message::Kill)
    .into()
}

/// Restart in warning amber: amber ink on an amber tint, with a faint amber ring.
fn restart_style() -> IconButtonStyle {
    IconButtonStyle {
        foreground: color::Accents::AMBER,
        foreground_hover: color::Accents::AMBER,
        background: color::Accents::AMBER_TINT,
        border: rgb!(255, 181, 69, 30%),
        background_hover: rgb!(255, 181, 69, 22%),
        border_hover: rgb!(255, 181, 69, 55%),
        foreground_disabled: color::Text::DISABLED,
        background_disabled: Color::TRANSPARENT,
        foreground_pressed: color::Accents::AMBER,
        background_pressed: rgb!(255, 181, 69, 26%),
    }
}

/// Ghost: no background and no border, as the new-process and console buttons have. Hover changes only
/// the icon's colour.
fn ghost_style() -> IconButtonStyle {
    IconButtonStyle {
        foreground: color::Text::MUTED,
        foreground_hover: color::Text::HEADLINE,
        background: Color::TRANSPARENT,
        border: Color::TRANSPARENT,
        background_hover: Color::TRANSPARENT,
        border_hover: Color::TRANSPARENT,
        foreground_disabled: color::Text::DISABLED,
        background_disabled: Color::TRANSPARENT,
        foreground_pressed: color::Text::HEADLINE,
        background_pressed: Color::TRANSPARENT,
    }
}

/// The Launch button. Without a message it is disabled, as it is while the game is prepared.
fn launch_button(label: String, on_press: Option<Message>) -> Element<'static, Message> {
    let button = TextButton::content(
        row![
            utility::icon(Icon::lucide().play(), 12.0, Some((color::Primary::ON, color::Primary::ON))),
            Text::new(label).size(14).font(fonts::Inter::BOLD).color(color::Primary::ON),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .variant(Variant::Primary)
    .padding(Padding::new(0.0).left(22).right(22).top(11).bottom(11))
    .disabled(on_press.is_none());
    match on_press {
        Some(message) => button.on_press(message).into(),
        None => button.into(),
    }
}

/// A 36px glass chrome button over the masthead art.
fn glass_button(markup: &'static str, message: Message) -> Element<'static, Message> {
    glass(
        IconButton::new(markup)
            .size(36)
            .radius(12.0)
            .icon_size(14.0)
            .style(glass_style())
            .on_press(message),
    )
}

/// The ⋯ menu in the glass frame: the instance's rows. Reveal is listed only for an instance from an
/// external source, since the folder button reveals the rest.
fn glass_menu(instance: &Instance, installing: bool) -> Element<'static, Message> {
    let menu = instance_menu::menu(installing, instance.source_url.is_some());
    let actions = menu.actions;
    glass(
        Dropdown::new(Icon::lucide().ellipsis_vertical())
            .items(menu.items)
            .width(36)
            .height(36)
            .on_select(move |index| Message::Menu(actions.get(index).copied().flatten())),
    )
}

/// The frame the glass buttons sit in: a dark translucent fill with a hairline ring.
fn glass<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
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

/// The status badge: launched while the game runs, preparing while it is prepared, idle otherwise.
fn status_badge(status: LaunchStatus) -> Element<'static, Message> {
    let (label, ink, fill, pulsing) = match status {
        LaunchStatus::Running => (lang::t("instance.status.launched"), color::Primary::DEFAULT, color::Primary::TINT, true),
        LaunchStatus::Preparing(_) => (
            lang::t("instance.status.preparing"),
            color::Accents::AMBER,
            color::Accents::AMBER_TINT,
            true,
        ),
        LaunchStatus::Idle => (lang::t("instance.status.idle"), color::Text::MUTED, rgb!(255, 255, 255, 6%), false),
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
