//! The pieces the resource page's views share: chips, stats, glyphs, panels, pictures and the buttons its actions use.

use aurora_leaf::Icon;
use iced::widget::{Text, container, image, row};
use iced::{Background, Border, Color, Element, Padding, alignment};
use lodestone_platforms::ContentType;

use super::{Action, Message};
use crate::app::components::button::Variant;
use crate::app::components::hint::hint;
use crate::app::components::icon_button::secondary_button_style;
use crate::app::components::{IconButton, TextButton, panel};
use crate::app::lang;
use crate::app::pages::discover::ProjectStatus;
use crate::app::theme::{color, fonts, utility};

/// A small rounded label: a tag, a loader or a Minecraft version.
pub(super) fn chip<'a, M: 'a>(label: impl Into<String>) -> Element<'a, M> {
    container(
        Text::new(label.into())
            .size(11)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted()),
    )
    .padding(Padding::new(0.0).left(8).right(8).top(3).bottom(3))
    .style(|_| container::Style {
        background: Some(Background::Color(color::surface_bg3())),
        border: Border {
            radius: 999.0.into(),
            width: 1.0,
            color: color::surface_line(),
        },
        ..container::Style::default()
    })
    .into()
}

/// A line icon in `tint`.
pub(super) fn glyph<'a, M: 'a>(markup: &'static str, size: f32, tint: Color) -> Element<'a, M> {
    utility::icon(markup, size, Some((tint, tint)))
}

/// A figure with the glyph that names it, as the header and the version rows show their counts.
pub(super) fn stat<'a, M: 'a>(markup: &'static str, figure: String) -> Element<'a, M> {
    row![
        glyph(markup, 12.0, color::text_muted()),
        Text::new(figure).size(12).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
    ]
    .spacing(6)
    .align_y(alignment::Vertical::Center)
    .into()
}

/// A section of the page in the shared panel.
pub(super) fn section<'a, M: 'a>(content: impl Into<Element<'a, M>>) -> Element<'a, M> {
    panel::panel_with_padding(content, Padding::new(18.0))
}

/// A section's heading: a small label above its content.
pub(super) fn heading<'a, M: 'a>(label: &'static str) -> Element<'a, M> {
    Text::new(label)
        .size(10)
        .font(fonts::JetBrainsMono::SEMI_BOLD)
        .color(color::text_placeholder())
        .into()
}

/// A project's picture as a square of `size`: the picture when it has loaded, else the package glyph on a plain square.
pub(super) fn picture<'a, M: 'a>(handle: Option<&image::Handle>, size: f32) -> Element<'a, M> {
    match handle {
        Some(handle) => image(handle.clone()).width(size).height(size).into(),
        None => container(glyph(Icon::lucide().package(), size * 0.4, color::text_muted()))
            .width(size)
            .height(size)
            .align_x(alignment::Horizontal::Center)
            .align_y(alignment::Vertical::Center)
            .style(|_| container::Style {
                background: Some(Background::Color(color::surface_bg3())),
                border: Border {
                    radius: 12.0.into(),
                    width: 0.0,
                    color: color::surface_line(),
                },
                ..container::Style::default()
            })
            .into(),
    }
}

/// A filled button with an icon: the action the page is for.
pub(super) fn primary<'a>(label: impl Into<String>, markup: &'static str, action: Action) -> Element<'a, Message> {
    TextButton::content(
        row![
            glyph(markup, 12.0, color::primary_on()),
            Text::new(label.into()).size(12).font(fonts::Inter::BOLD).color(color::primary_on()),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    )
    .variant(Variant::Primary)
    .padding(Padding::new(0.0).left(14).right(14).top(8).bottom(8))
    .on_press(Message::Act(action))
    .into()
}

/// A dimmed label standing in for an action that cannot be taken now.
pub(super) fn dimmed<'a>(label: &'static str) -> Element<'a, Message> {
    TextButton::content(Text::new(label).size(12).font(fonts::Inter::MEDIUM).color(color::text_muted()))
        .variant(Variant::Ghost)
        .padding(Padding::new(0.0).left(14).right(14).top(8).bottom(8))
        .disabled(true)
        .into()
}

/// A loader's name as it is shown: `fabric` is `Fabric`, and `neoforge` is `NeoForge`.
pub(super) fn loader_label(name: &str) -> String {
    match name {
        "neoforge" => "NeoForge".to_string(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        }
    }
}

/// The button a row of the versions takes that release: install a modpack, add a mod, or download anything else.
pub(super) fn row_action<'a>(content: ContentType, version_id: &str) -> Element<'a, Message> {
    let (markup, label, action) = match content {
        ContentType::Modpack => (
            Icon::lucide().download(),
            lang::t("resource.action.install"),
            Action::Install(Some(version_id.to_string())),
        ),
        ContentType::Mod => (
            Icon::lucide().plus(),
            lang::t("resource.action.add"),
            Action::Add(Some(version_id.to_string())),
        ),
        ContentType::ResourcePack | ContentType::Shader | ContentType::DataPack | ContentType::World => (
            Icon::lucide().download(),
            lang::t("resource.action.download"),
            Action::Download(Some(version_id.to_string())),
        ),
    };
    hint(
        IconButton::new(markup)
            .size(32.0)
            .radius(10.0)
            .icon_size(14.0)
            .style(secondary_button_style())
            .on_press(Message::Act(action)),
        label,
    )
}

/// Where a mod's release stands in the instance the Mods tab picked. It decides what the release's action offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ModState {
    /// The release is installed in the instance.
    Installed,
    /// Another release of the mod is installed, and this one can take its place.
    Switch,
    /// The release, or another release of the mod, is being added or replaced now.
    Adding,
    /// The release can be added.
    Add,
    /// The mod is installed, and no release was asked about.
    Added,
    /// No instance takes mods, so nothing can be added.
    Unavailable,
}

/// The state of a release of a mod, given what the target instance has of the mod. `version` is the release the action
/// takes, or `None` when it takes the newest that fits.
pub(super) fn mod_state(status: &ProjectStatus, version: Option<&str>) -> ModState {
    if !status.target.as_ref().is_some_and(|target| target.takes_mods()) {
        ModState::Unavailable
    } else if status.added_version.is_some() && status.added_version.as_deref() == version {
        ModState::Installed
    } else if status.adding {
        ModState::Adding
    } else if status.added && version.is_some() {
        ModState::Switch
    } else if status.added {
        ModState::Added
    } else {
        ModState::Add
    }
}

/// The button a versions row takes a mod's release with: a check when the release is installed, a switch when another
/// release is, the plus when it can be added, and a dimmed plus when it cannot.
pub(super) fn mod_row_action<'a>(state: ModState, version_id: &str) -> Element<'a, Message> {
    let button = |icon: &'static str| {
        IconButton::new(icon)
            .size(32.0)
            .radius(10.0)
            .icon_size(14.0)
            .style(secondary_button_style())
    };
    match state {
        ModState::Installed | ModState::Added => hint(button(Icon::lucide().check()).disabled(true), lang::t("resource.action.added")),
        ModState::Adding => hint(button(Icon::lucide().plus()).disabled(true), lang::t("resource.action.adding")),
        ModState::Switch => hint(
            button(Icon::lucide().repeat()).on_press(Message::Act(Action::Replace(version_id.to_string()))),
            lang::t("resource.action.switch"),
        ),
        ModState::Add => hint(
            button(Icon::lucide().plus()).on_press(Message::Act(Action::Add(Some(version_id.to_string())))),
            lang::t("resource.action.add"),
        ),
        ModState::Unavailable => hint(button(Icon::lucide().plus()).disabled(true), lang::t("resource.action.add")),
    }
}

/// Previous and next buttons with the page on screen between them. Each button sends `on_page` with the page it
/// moves to, and a button at either end of the list stands dimmed.
pub(super) fn pager<'a>(current: usize, pages: usize, on_page: fn(usize) -> Message) -> Element<'a, Message> {
    let previous = IconButton::new(Icon::lucide().chevron_left())
        .size(32.0)
        .radius(10.0)
        .icon_size(14.0)
        .style(secondary_button_style());
    let previous = if current > 0 {
        previous.on_press(on_page(current - 1))
    } else {
        previous.disabled(true)
    };
    let next = IconButton::new(Icon::lucide().chevron_right())
        .size(32.0)
        .radius(10.0)
        .icon_size(14.0)
        .style(secondary_button_style());
    let next = if current + 1 < pages {
        next.on_press(on_page(current + 1))
    } else {
        next.disabled(true)
    };
    let position = lang::t_with(
        "resource.pager.position",
        &[("page", (current + 1).to_string().as_str()), ("pages", pages.to_string().as_str())],
    );
    row![
        hint(previous, lang::t("resource.pager.previous")),
        Text::new(position)
            .size(12)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::text_muted()),
        hint(next, lang::t("resource.pager.next")),
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .into()
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/resource/parts.rs"]
mod tests;
