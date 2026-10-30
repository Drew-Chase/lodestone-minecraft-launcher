//! The section bodies of the Settings tab. Each is a divided panel of setting
//! rows; the danger zone sits in a red-bordered panel.

use iced::widget::{Space, Text, column, container, rich_text, row, slider, span, text_input};
use iced::{Background, Border, Element, Fill, Padding, alignment};
use iced_color_extension::rgb;

use crate::app::components::badge::{Badge, loader_palette};
use crate::app::components::button::Variant;
use crate::app::components::panel::{bordered_panel, divided, divided_panel};
use crate::app::components::segmented::segmented;
use crate::app::components::setting_row::{described_setting, setting};
use crate::app::components::slider::style as slider_style;
use crate::app::components::{Select, TextButton, Toggle, anim, input};
use crate::app::lang;
use crate::app::pages::library::instance::Instance;
use crate::app::theme::{color, fonts};

use super::{
    Axis, Choice, GARBAGE_COLLECTORS, JAVA_RUNTIMES, MEMORY_RANGE, MINECRAFT_VERSIONS, Message, SYSTEM_MEMORY_GB, Section, Settings, Switch,
    launch_modes, parse_dimension,
};

/// The body of `section`, built from the tab's settings and the instance.
pub fn body<'a>(section: Section, settings: &'a Settings, instance: &'a Instance) -> Element<'a, Message> {
    match section {
        Section::JavaMemory => java_memory(settings),
        Section::Window => window(settings),
        Section::LaunchArgs => launch_args(settings),
        Section::GameVersion => game_version(settings, instance),
        Section::PathsFiles => paths_files(instance),
        Section::Advanced => advanced(settings),
        Section::DangerZone => danger_zone(),
    }
}

/// Horizontal padding inside every section panel.
fn panel_padding() -> Padding {
    Padding::new(0.0).left(18.0).right(18.0)
}

/// A switch that reports its new value under `switch`.
fn toggle(on: bool, switch: Switch) -> Toggle<Message> {
    Toggle::new(on).on_toggle_with(move |value| Message::Switch(switch, value))
}

fn java_memory(settings: &Settings) -> Element<'_, Message> {
    let java = Select::new(JAVA_RUNTIMES, Some(settings.java_runtime))
        .on_select(|index| Message::Choose(Choice::JavaRuntime, index))
        .width(200);
    let collector = Select::new(GARBAGE_COLLECTORS, Some(settings.garbage_collector))
        .on_select(|index| Message::Choose(Choice::GarbageCollector, index))
        .width(200);
    let memory = row![
        slider(
            f32::from(*MEMORY_RANGE.start())..=f32::from(*MEMORY_RANGE.end()),
            f32::from(settings.memory_gb),
            Message::Memory,
        )
        .step(1.0_f32)
        .style(slider_style)
        .width(200),
        Text::new(format!("{} GB", settings.memory_gb))
            .size(12)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::Primary::DEFAULT),
    ]
    .spacing(14)
    .align_y(alignment::Vertical::Center);
    let allocation = lang::t_with(
        "instance.settings.memory_hint",
        &[
            ("allocated", settings.memory_gb.to_string().as_str()),
            ("total", SYSTEM_MEMORY_GB.to_string().as_str()),
        ],
    );

    divided_panel(
        vec![
            described_setting(
                lang::t("instance.settings.java_runtime"),
                lang::t("instance.settings.java_runtime_hint"),
                java,
            ),
            described_setting(lang::t("instance.settings.allocated_memory"), allocation, memory),
            described_setting(
                lang::t("instance.settings.garbage_collector"),
                lang::t("instance.settings.garbage_collector_hint"),
                collector,
            ),
            setting(lang::t("instance.settings.system_java"), toggle(settings.system_java, Switch::SystemJava)),
            jvm_arguments(),
        ],
        panel_padding(),
    )
}

/// The JVM arguments as a code block, each flag coloured by what it tunes.
fn jvm_arguments() -> Element<'static, Message> {
    let flags = rich_text::<(), Message, iced::Theme, iced::Renderer>([
        span("-XX:+UseG1GC ").color(color::Primary::DEFAULT),
        span("-XX:+ParallelRefProcEnabled ").color(color::Text::DEFAULT),
        span("-XX:MaxGCPauseMillis=200 ").color(color::Accents::AMBER),
        span("-XX:+UnlockExperimentalVMOptions -XX:G1NewSizePercent=30 ").color(color::Text::DEFAULT),
        span("-Dfml.ignoreInvalidMinecraftCertificates=true").color(color::Accents::CYAN),
    ])
    .font(fonts::JetBrainsMono::REGULAR)
    .size(11);

    column![
        Text::new(lang::t("instance.settings.jvm_arguments"))
            .size(10)
            .font(fonts::JetBrainsMono::REGULAR)
            .color(color::Text::PLACEHOLDER),
        container(flags)
            .width(Fill)
            .padding(Padding::new(0.0).left(12).right(12).top(10).bottom(10))
            .style(|_| container::Style {
                background: Some(Background::Color(rgb!(0, 0, 0, 30%))),
                border: Border {
                    radius: 10.0.into(),
                    width: 1.0,
                    color: color::Surface::LINE,
                },
                ..container::Style::default()
            }),
    ]
    .spacing(8)
    .padding(Padding::new(0.0).top(14).bottom(14))
    .into()
}

fn window(settings: &Settings) -> Element<'_, Message> {
    let valid = parse_dimension(&settings.width).is_some() && parse_dimension(&settings.height).is_some();
    let resolution = row![
        dimension_input(&settings.width, valid, Axis::Width),
        Text::new("×").size(12).font(fonts::Inter::REGULAR).color(color::Text::MUTED),
        dimension_input(&settings.height, valid, Axis::Height),
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Center);

    divided_panel(
        vec![
            setting(
                lang::t("instance.settings.launch_mode_label"),
                segmented(&launch_modes(), settings.launch_mode, Message::LaunchMode),
            ),
            setting(lang::t("instance.settings.resolution"), resolution),
            described_setting(
                lang::t("instance.settings.hide_launcher"),
                lang::t("instance.settings.hide_launcher_hint"),
                toggle(settings.hide_launcher, Switch::HideLauncher),
            ),
            setting(
                lang::t("instance.settings.quit_after_exit"),
                toggle(settings.quit_after_exit, Switch::QuitAfterExit),
            ),
        ],
        panel_padding(),
    )
}

/// A width or height field: digits only, outlined red while the pair is not
/// a valid resolution.
fn dimension_input(value: &str, valid: bool, axis: Axis) -> Element<'_, Message> {
    text_input("", value)
        .on_input(move |text| Message::Dimension(axis, text))
        .style(move |theme, status| {
            let mut style = input::style(theme, status);
            if !valid {
                style.border.color = color::buttons::Danger::BACKGROUND_HOVER;
            }
            style
        })
        .font(fonts::JetBrainsMono::REGULAR)
        .size(13)
        .padding(Padding::new(0.0).left(12).right(12).top(8).bottom(8))
        .width(96)
        .into()
}

fn launch_args(settings: &Settings) -> Element<'_, Message> {
    let arguments = text_input("--width 1280", &settings.game_args)
        .on_input(Message::GameArgs)
        .style(input::style)
        .font(fonts::JetBrainsMono::REGULAR)
        .size(13)
        .padding(Padding::new(0.0).left(12).right(12).top(8).bottom(8))
        .width(360);

    divided_panel(
        vec![described_setting(
            lang::t("instance.settings.game_arguments"),
            lang::t("instance.settings.game_arguments_hint"),
            arguments,
        )],
        panel_padding(),
    )
}

fn game_version<'a>(settings: &'a Settings, instance: &'a Instance) -> Element<'a, Message> {
    let (ink, fill) = loader_palette(instance.loader);
    let minecraft = Select::new(MINECRAFT_VERSIONS, Some(settings.minecraft_version))
        .on_select(|index| Message::Choose(Choice::MinecraftVersion, index))
        .width(160);
    let loader = row![
        Badge::new(instance.loader.label(), ink, fill).with_loader(instance.loader),
        Select::new(&settings.loader_versions, Some(settings.loader_version))
            .on_select(|index| Message::Choose(Choice::LoaderVersion, index))
            .width(160),
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Center);

    divided_panel(
        vec![
            setting("Minecraft", minecraft),
            setting(lang::t("instance.settings.mod_loader"), loader),
            setting(lang::t("instance.settings.snapshots"), toggle(settings.snapshots, Switch::Snapshots)),
        ],
        panel_padding(),
    )
}

fn paths_files(instance: &Instance) -> Element<'_, Message> {
    let folder = format!("/instances/{}", instance.directory_name);
    divided_panel(
        vec![described_setting(
            lang::t("instance.settings.instance_folder"),
            folder,
            TextButton::ghost(lang::t("common.reveal")).on_press(Message::RevealFolder),
        )],
        panel_padding(),
    )
}

fn advanced(settings: &Settings) -> Element<'_, Message> {
    divided_panel(
        vec![
            described_setting(
                lang::t("instance.settings.env_vars"),
                lang::t("instance.settings.env_vars_hint"),
                TextButton::ghost(lang::t_with(
                    "instance.settings.env_set",
                    &[("count", settings.environment_variables.to_string().as_str())],
                ))
                .on_press(Message::EditEnvironment),
            ),
            described_setting(
                lang::t("instance.settings.pre_launch_hook"),
                lang::t("instance.settings.pre_launch_hook_hint"),
                TextButton::ghost(lang::t("instance.settings.add_script")).on_press(Message::AddScript),
            ),
            described_setting(
                lang::t("instance.settings.verbose_logging"),
                lang::t("instance.settings.verbose_logging_hint"),
                toggle(settings.verbose_logging, Switch::VerboseLogging),
            ),
            setting(
                lang::t("instance.settings.skip_integrity"),
                toggle(settings.skip_integrity, Switch::SkipIntegrity),
            ),
        ],
        panel_padding(),
    )
}

/// The destructive actions in a red-bordered panel. None of them run yet.
fn danger_zone() -> Element<'static, Message> {
    let red = color::buttons::Danger::BACKGROUND_HOVER;
    let delete = row![
        column![
            Text::new(lang::t("instance.settings.delete_title"))
                .size(13)
                .font(fonts::Inter::MEDIUM)
                .color(red),
            Text::new(lang::t("instance.settings.delete_hint"))
                .size(11)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED),
        ]
        .spacing(3),
        Space::new().width(Fill),
        TextButton::ghost(lang::t("common.delete"))
            .variant(Variant::Danger)
            .on_press(Message::DeleteInstance),
    ]
    .spacing(16)
    .align_y(alignment::Vertical::Center)
    .padding(Padding::new(0.0).top(14).bottom(14));

    bordered_panel(
        divided(vec![
            described_setting(
                lang::t("instance.settings.reinstall_title"),
                lang::t("instance.settings.reinstall_hint"),
                TextButton::ghost(lang::t("instance.settings.reinstall")).on_press(Message::Reinstall),
            ),
            described_setting(
                lang::t("instance.settings.reset_title"),
                lang::t("instance.settings.reset_hint"),
                TextButton::ghost(lang::t("instance.settings.reset")).on_press(Message::ResetSettings),
            ),
            delete.into(),
        ]),
        panel_padding(),
        anim::fade(red, 0.3),
    )
}
