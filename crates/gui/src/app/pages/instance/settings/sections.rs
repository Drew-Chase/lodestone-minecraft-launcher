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
use crate::app::pages::library::instance::Instance;
use crate::app::theme::{color, fonts};

use super::{
    Axis, Choice, GARBAGE_COLLECTORS, JAVA_RUNTIMES, LAUNCH_MODES, MEMORY_RANGE, MINECRAFT_VERSIONS, Message, SYSTEM_MEMORY_GB, Section, Settings,
    Switch, parse_dimension,
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
    let allocation = format!(
        "{} GB of {SYSTEM_MEMORY_GB} GB available · Minecraft recommends 4–8 GB for modded.",
        settings.memory_gb
    );

    divided_panel(
        vec![
            described_setting("Java runtime", "Auto-detected from your system installations.", java),
            described_setting("Allocated memory", allocation, memory),
            described_setting(
                "Garbage collector",
                "G1GC is recommended for most modpacks. Use ZGC for pause-sensitive gameplay.",
                collector,
            ),
            setting("Use system Java instead of bundled", toggle(settings.system_java, Switch::SystemJava)),
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
        Text::new("JVM ARGUMENTS")
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
            setting("Launch mode", segmented(&LAUNCH_MODES, settings.launch_mode, Message::LaunchMode)),
            setting("Resolution", resolution),
            described_setting(
                "Hide launcher while playing",
                "Minimize Lodestone until you quit the game.",
                toggle(settings.hide_launcher, Switch::HideLauncher),
            ),
            setting("Quit launcher after game exits", toggle(settings.quit_after_exit, Switch::QuitAfterExit)),
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
            "Game arguments",
            "Appended after the default arguments when the game launches.",
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
            setting("Mod loader", loader),
            setting("Include snapshots and pre-releases", toggle(settings.snapshots, Switch::Snapshots)),
        ],
        panel_padding(),
    )
}

fn paths_files(instance: &Instance) -> Element<'_, Message> {
    let folder = format!("/instances/{}", instance.directory_name);
    divided_panel(
        vec![described_setting(
            "Instance folder",
            folder,
            TextButton::ghost("Reveal").on_press(Message::RevealFolder),
        )],
        panel_padding(),
    )
}

fn advanced(settings: &Settings) -> Element<'_, Message> {
    divided_panel(
        vec![
            described_setting(
                "Environment variables",
                "Passed to the JVM process at launch.",
                TextButton::ghost(format!("{} set · Edit", settings.environment_variables)).on_press(Message::EditEnvironment),
            ),
            described_setting(
                "Pre-launch hook",
                "Run a shell command before the game starts.",
                TextButton::ghost("Add script").on_press(Message::AddScript),
            ),
            described_setting(
                "Verbose logging",
                "Emit DEBUG-level output to the console.",
                toggle(settings.verbose_logging, Switch::VerboseLogging),
            ),
            setting("Skip integrity check on launch", toggle(settings.skip_integrity, Switch::SkipIntegrity)),
        ],
        panel_padding(),
    )
}

/// The destructive actions in a red-bordered panel. None of them run yet.
fn danger_zone() -> Element<'static, Message> {
    let red = color::buttons::Danger::BACKGROUND_HOVER;
    let delete = row![
        column![
            Text::new("Delete instance").size(13).font(fonts::Inter::MEDIUM).color(red),
            Text::new("Permanently remove this instance and all of its worlds, mods, and screenshots.")
                .size(11)
                .font(fonts::Inter::REGULAR)
                .color(color::Text::MUTED),
        ]
        .spacing(3),
        Space::new().width(Fill),
        TextButton::ghost("Delete").variant(Variant::Danger).on_press(Message::DeleteInstance),
    ]
    .spacing(16)
    .align_y(alignment::Vertical::Center)
    .padding(Padding::new(0.0).top(14).bottom(14));

    bordered_panel(
        divided(vec![
            described_setting(
                "Reinstall game files",
                "Redownload vanilla Minecraft jars for this version. Mods and worlds are preserved.",
                TextButton::ghost("Reinstall").on_press(Message::Reinstall),
            ),
            described_setting(
                "Reset settings to defaults",
                "Revert this instance back to global defaults.",
                TextButton::ghost("Reset").on_press(Message::ResetSettings),
            ),
            delete.into(),
        ]),
        panel_padding(),
        anim::fade(red, 0.3),
    )
}
