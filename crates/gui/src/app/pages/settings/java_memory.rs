//! The Java & Memory tab: the heap every instance gets and its garbage collector, the runtimes an instance can run on, the
//! JVM arguments, and the game arguments and advanced options an instance inherits.

use iced::widget::{Space, column, container, row, slider, text, text_editor};
use iced::{Background, Border, Element, Fill, Length, Padding, alignment};

use super::{Message, Runtime, card};
use crate::app::components::TextButton;
use crate::app::components::launch_rows::{self, MIN_MEMORY_GB, Shown};
use crate::app::components::panel::divided;
use crate::app::lang;
use crate::app::launch_settings::{JavaChoice, LaunchChange};
use crate::app::theme::{color, fonts};

/// The heap the Memory allocation caption recommends, in gigabytes.
const RECOMMENDED_GB: u32 = 8;

/// The tab: the memory card, the runtimes, the JVM arguments, the game arguments and the advanced options.
pub fn view<'a>(shown: Shown<'a>, jvm: &'a text_editor::Content, runtimes: &'a [Runtime]) -> Element<'a, Message> {
    column![
        card(
            lang::t("settings.java_memory.memory_title"),
            lang::t("settings.java_memory.memory_subtitle"),
            memory(shown),
        ),
        card(
            lang::t("settings.java_memory.runtimes_title"),
            lang::t("settings.java_memory.runtimes_subtitle"),
            divided(runtime_rows(shown, runtimes)),
        ),
        card(
            lang::t("instance.settings.jvm_arguments"),
            lang::t("settings.java_memory.jvm_subtitle"),
            jvm_editor(jvm),
        ),
        card(
            lang::t("instance.settings.game_arguments"),
            lang::t("instance.settings.game_arguments_hint"),
            divided(launch_rows::launch_arg_rows(shown, Message::Change)),
        ),
        card(
            lang::t("instance.settings.section.advanced"),
            lang::t("settings.defaults.subtitle.advanced"),
            divided(launch_rows::advanced_rows(shown, Message::Change, Message::OpenEnvironment)),
        ),
    ]
    .spacing(24)
    .width(Fill)
    .into()
}

/// The heap slider, its captions, and the garbage collector beneath them.
fn memory<'a>(shown: Shown<'a>) -> Element<'a, Message> {
    let gb = shown.values.memory_mb / 1024;
    let ceiling = shown.ceiling_gb.max(MIN_MEMORY_GB);
    let bar = slider(MIN_MEMORY_GB as f32..=ceiling as f32, gb as f32, |value| {
        Message::Change(LaunchChange::Memory(value.round() as u32 * 1024))
    })
    .step(1.0_f32)
    .style(crate::app::components::slider::style)
    .width(Fill);
    let value = text(lang::t_with("instance.settings.gigabytes", &[("gb", gb.to_string().as_str())]))
        .size(16)
        .font(fonts::JetBrainsMono::SEMI_BOLD)
        .color(color::primary());
    let captions = row![
        caption(lang::t_with(
            "settings.java_memory.caption_min",
            &[("gb", MIN_MEMORY_GB.to_string().as_str())]
        )),
        Space::new().width(Fill),
        caption(lang::t_with(
            "settings.java_memory.caption_recommended",
            &[("gb", RECOMMENDED_GB.to_string().as_str())],
        )),
        Space::new().width(Fill),
        caption(lang::t_with("settings.java_memory.caption_max", &[("gb", ceiling.to_string().as_str())])),
    ]
    .align_y(alignment::Vertical::Center);
    column![
        row![bar, value].spacing(16).align_y(alignment::Vertical::Center),
        captions,
        launch_rows::collector_row(shown, Message::Change),
    ]
    .spacing(14)
    .width(Fill)
    .into()
}

/// One muted caption under the heap slider.
fn caption<'a>(label: String) -> Element<'a, Message> {
    text(label).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()).into()
}

/// The rows of the Installed runtimes list: Follow the version, the installed bundled runtimes, and the system Java.
/// The row the launch default names shows a DEFAULT badge, and the others a Use button.
fn runtime_rows<'a>(shown: Shown<'a>, runtimes: &'a [Runtime]) -> Vec<Element<'a, Message>> {
    let current = &shown.values.java;
    let mut rows = vec![runtime_row(
        lang::t("instance.settings.java_follow").to_owned(),
        lang::t("settings.java_memory.follow_hint").to_owned(),
        JavaChoice::Follow,
        *current == JavaChoice::Follow,
    )];
    for runtime in runtimes {
        rows.push(runtime_row(
            runtime.label.clone(),
            runtime.path.clone(),
            runtime.choice.clone(),
            *current == runtime.choice,
        ));
    }
    if let Some(major) = shown.java.system {
        rows.push(runtime_row(
            lang::t_with("instance.settings.java_system", &[("major", major.to_string().as_str())]),
            lang::t("settings.java_memory.system_hint").to_owned(),
            JavaChoice::System,
            *current == JavaChoice::System,
        ));
    }
    rows
}

/// One runtime: its name and where it lives, then a DEFAULT badge when it is the launch default, or a Use button.
fn runtime_row<'a>(label: String, detail: String, choice: JavaChoice, is_default: bool) -> Element<'a, Message> {
    let name = column![
        text(label).size(14).font(fonts::Inter::MEDIUM).color(color::text_headline()),
        text(detail).size(11).font(fonts::JetBrainsMono::REGULAR).color(color::text_muted()),
    ]
    .spacing(3);
    let control: Element<'a, Message> = if is_default {
        default_badge()
    } else {
        TextButton::ghost(lang::t("settings.java_memory.use"))
            .on_press(Message::Change(LaunchChange::Java(choice)))
            .into()
    };
    container(
        row![name, Space::new().width(Fill), control]
            .spacing(16)
            .align_y(alignment::Vertical::Center),
    )
    .width(Fill)
    .padding(Padding::new(0.0).top(12.0).bottom(12.0).left(14.0).right(14.0))
    .style(move |_| container::Style {
        background: is_default.then(|| Background::Color(color::primary_tint())),
        border: Border {
            radius: 10.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}

/// The badge that marks the runtime the launch default uses.
fn default_badge<'a>() -> Element<'a, Message> {
    container(
        text(lang::t("settings.java_memory.default"))
            .size(10)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::primary()),
    )
    .padding(Padding::new(0.0).left(8.0).right(8.0).top(3.0).bottom(3.0))
    .style(|_| container::Style {
        background: Some(Background::Color(color::primary_tint())),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: color::primary_glow(),
        },
        ..container::Style::default()
    })
    .into()
}

/// The JVM arguments, in a monospace editor with the green ink of a code block.
fn jvm_editor<'a>(content: &'a text_editor::Content) -> Element<'a, Message> {
    text_editor(content)
        .placeholder(lang::t("instance.settings.jvm_placeholder"))
        .on_action(Message::JvmEdit)
        .font(fonts::JetBrainsMono::REGULAR)
        .size(13)
        .height(Length::Fixed(96.0))
        .padding(Padding::new(14.0))
        .style(code_style)
        .into()
}

/// The code block's look: a dark well with a hairline border, brighter while it has focus.
fn code_style(_theme: &iced::Theme, status: text_editor::Status) -> text_editor::Style {
    let focused = matches!(status, text_editor::Status::Focused { .. });
    text_editor::Style {
        background: Background::Color(color::surface_bg0()),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: if focused { color::primary_glow() } else { color::surface_line() },
        },
        placeholder: color::text_placeholder(),
        value: color::primary(),
        selection: color::primary_tint(),
    }
}
