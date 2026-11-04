//! The Appearance tab: the accent the interface highlights with, the surface theme, and the motion and effects it uses.

use aurora_leaf::Icon;
use iced::widget::{Row, Space, button, column, container, text, text_input};
use iced::{Background, Border, Color, Element, Fill, Padding, alignment};

use super::edit::Edit;
use super::{Message, card};
use crate::app::components::Toggle;
use crate::app::components::input;
use crate::app::components::panel::divided;
use crate::app::components::setting_row::described_setting;
use crate::app::lang;
use crate::app::preferences::{Appearance, SettingsValues};
use crate::app::theme::{Accent, SurfaceTheme, color, fonts, parse_hex, utility};

/// The accent swatches, in the order the tab shows them.
const SWATCHES: [Accent; 6] = [Accent::Green, Accent::Violet, Accent::Amber, Accent::Cyan, Accent::Pink, Accent::Orange];

/// The surface themes, in the order the tab shows them.
const THEMES: [SurfaceTheme; 3] = [SurfaceTheme::Void, SurfaceTheme::Slate, SurfaceTheme::Obsidian];

/// The side of a swatch, in pixels.
const SWATCH_SIZE: f32 = 44.0;

/// The Appearance tab: the accent, the surface theme, and the motion and effects switches.
pub fn view<'a>(settings: &'a SettingsValues, custom_hex: &'a str) -> Element<'a, Message> {
    column![
        card(
            lang::t("settings.appearance.accent_title"),
            lang::t("settings.appearance.accent_subtitle"),
            accent_body(&settings.appearance.accent, custom_hex),
        ),
        card(
            lang::t("settings.appearance.theme_title"),
            lang::t("settings.appearance.theme_subtitle"),
            theme_body(settings.appearance.theme),
        ),
        card(
            lang::t("settings.appearance.nav_title"),
            lang::t("settings.appearance.nav_subtitle"),
            divided(navigation_rows(&settings.appearance)),
        ),
        card(
            lang::t("settings.appearance.effects_title"),
            lang::t("settings.appearance.effects_subtitle"),
            divided(effect_rows(&settings.appearance)),
        ),
    ]
    .spacing(24)
    .width(Fill)
    .into()
}

/// The swatches, then the field for the custom color while the custom accent is in use.
fn accent_body<'a>(accent: &Accent, custom_hex: &'a str) -> Element<'a, Message> {
    let custom = matches!(accent, Accent::Custom { .. });
    let mut swatches = Row::new().spacing(12).align_y(alignment::Vertical::Center);
    for swatch in SWATCHES {
        let selected = *accent == swatch;
        swatches = swatches.push(swatch_button(swatch, selected));
    }
    swatches = swatches.push(custom_button(custom, custom_hex));
    let body = column![swatches].spacing(16);
    if custom {
        body.push(custom_field(custom_hex)).into()
    } else {
        body.into()
    }
}

/// One swatch: its color, a check mark when it is the accent in use, and a press that picks it.
fn swatch_button<'a>(accent: Accent, selected: bool) -> Element<'a, Message> {
    let fill = accent.color();
    let mark: Element<'a, Message> = if selected {
        utility::icon(Icon::lucide().check(), 16.0, Some((color::surface_bg0(), color::surface_bg0())))
    } else {
        Space::new().into()
    };
    button(centered(mark))
        .width(SWATCH_SIZE)
        .height(SWATCH_SIZE)
        .padding(0)
        .on_press(Message::Edit(Edit::Accent(accent)))
        .style(move |_, _| button::Style {
            background: Some(Background::Color(fill)),
            border: Border {
                radius: 12.0.into(),
                width: if selected { 2.0 } else { 0.0 },
                color: color::primary(),
            },
            ..button::Style::default()
        })
        .into()
}

/// The "+" tile: it picks the custom accent, which is the color typed in the field.
fn custom_button<'a>(selected: bool, custom_hex: &'a str) -> Element<'a, Message> {
    let ink = if selected { color::primary() } else { color::text_muted() };
    button(centered(utility::icon(Icon::lucide().plus(), 16.0, Some((ink, ink)))))
        .width(SWATCH_SIZE)
        .height(SWATCH_SIZE)
        .padding(0)
        .on_press(Message::Edit(Edit::Accent(Accent::Custom { hex: custom_hex.to_owned() })))
        .style(move |_, _| button::Style {
            background: Some(Background::Color(color::surface_bg3())),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: if selected { color::primary() } else { color::surface_line_strong() },
            },
            ..button::Style::default()
        })
        .into()
}

/// The field that types the custom accent. It is outlined red while the text is not a `#rrggbb` color.
fn custom_field<'a>(custom_hex: &'a str) -> Element<'a, Message> {
    let valid = parse_hex(custom_hex).is_some();
    text_input(lang::t("settings.appearance.custom_hex_placeholder"), custom_hex)
        .on_input(Message::CustomHex)
        .style(move |theme, status| {
            let mut style = input::style(theme, status);
            if !valid {
                style.border.color = color::buttons::danger_background_hover();
            }
            style
        })
        .font(fonts::JetBrainsMono::REGULAR)
        .size(13)
        .padding(Padding::new(0.0).left(12.0).right(12.0).top(8.0).bottom(8.0))
        .width(200)
        .into()
}

/// The three surface themes as cards, the chosen one outlined in the accent.
fn theme_body<'a>(chosen: SurfaceTheme) -> Element<'a, Message> {
    Row::with_children(THEMES.map(|theme| theme_card(theme, theme == chosen)))
        .spacing(16)
        .width(Fill)
        .into()
}

/// One surface theme: a small preview of its canvas and panel, then its name.
fn theme_card<'a>(theme: SurfaceTheme, selected: bool) -> Element<'a, Message> {
    let (canvas, panel) = color::surface_preview(theme);
    let bar = container(Space::new().width(Fill).height(8)).style(move |_| fill_style(panel, 4.0));
    let dot = container(Space::new().width(22).height(22)).style(move |_| fill_style(color::primary(), 6.0));
    let block = container(Space::new().width(Fill).height(40)).style(move |_| fill_style(panel, 8.0));
    let preview = container(column![bar, Row::new().push(dot).push(block).spacing(8).align_y(alignment::Vertical::Center)].spacing(8))
        .width(Fill)
        .padding(10)
        .style(move |_| fill_style(canvas, 12.0));
    let outline = if selected { color::primary() } else { color::surface_line() };
    button(
        column![
            preview,
            text(theme_label(theme)).size(13).font(fonts::Inter::MEDIUM).color(color::text_headline()),
        ]
        .spacing(10),
    )
    .width(Fill)
    .padding(10)
    .on_press(Message::Edit(Edit::Theme(theme)))
    .style(move |_, _| button::Style {
        background: Some(Background::Color(color::surface_bg2())),
        border: Border {
            radius: 14.0.into(),
            width: 1.0,
            color: outline,
        },
        ..button::Style::default()
    })
    .into()
}

/// The name of a surface theme.
fn theme_label(theme: SurfaceTheme) -> &'static str {
    match theme {
        SurfaceTheme::Void => lang::t("settings.appearance.void"),
        SurfaceTheme::Slate => lang::t("settings.appearance.slate"),
        SurfaceTheme::Obsidian => lang::t("settings.appearance.obsidian"),
    }
}

/// The navigation switches: where the main navigation sits.
fn navigation_rows<'a>(appearance: &Appearance) -> Vec<Element<'a, Message>> {
    let on = appearance.floating_bottom_bar;
    vec![described_setting(
        lang::t("settings.appearance.floating_bar"),
        lang::t("settings.appearance.floating_bar_hint"),
        Toggle::new(on).on_toggle(Message::Edit(Edit::FloatingBottomBar(!on))),
    )]
}

/// The motion and effects switches.
fn effect_rows<'a>(appearance: &Appearance) -> Vec<Element<'a, Message>> {
    let switch = |on: bool, edit: fn(bool) -> Edit| Toggle::new(on).on_toggle(Message::Edit(edit(!on)));
    vec![
        described_setting(
            lang::t("settings.appearance.animations"),
            lang::t("settings.appearance.animations_hint"),
            switch(appearance.interface_animations, Edit::Animations),
        ),
        described_setting(
            lang::t("settings.appearance.particles"),
            lang::t("settings.appearance.particles_hint"),
            switch(appearance.ambient_particles, Edit::Particles),
        ),
        described_setting(
            lang::t("settings.appearance.glass"),
            lang::t("settings.appearance.glass_hint"),
            switch(appearance.glass_effect, Edit::Glass),
        ),
        described_setting(
            lang::t("settings.appearance.aurora"),
            lang::t("settings.appearance.aurora_hint"),
            switch(appearance.aurora_backgrounds, Edit::Aurora),
        ),
        described_setting(
            lang::t("settings.appearance.reduce_motion"),
            lang::t("settings.appearance.reduce_motion_hint"),
            switch(appearance.reduce_motion, Edit::ReduceMotion),
        ),
    ]
}

/// A flat fill with rounded corners.
fn fill_style(fill: Color, radius: f32) -> container::Style {
    container::Style {
        background: Some(Background::Color(fill)),
        border: Border {
            radius: radius.into(),
            ..Border::default()
        },
        ..container::Style::default()
    }
}

/// `content` centered in the space its parent gives it.
fn centered<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .width(Fill)
        .height(Fill)
        .align_x(alignment::Horizontal::Center)
        .align_y(alignment::Vertical::Center)
        .into()
}
