//! Settings tab — the instance's launch configuration, one section at a time.
//!
//! The sub-nav picks a section and its body fills the page. Controls are
//! addressed by enum rather than one message per field, and every value is
//! display state until the launch pipeline reads it.

mod sections;

use std::ops::RangeInclusive;

use aurora_leaf::Icon;
use iced::widget::{Text, column, container, row};
use iced::{Background, Border, Color, Element, Fill, Padding, alignment};
use iced_color_extension::rgb;

use crate::app::components::TextButton;
use crate::app::components::button::Variant;
use crate::app::components::setting_row::section_header;
use crate::app::pages::library::instance::Instance;
use crate::app::theme::{color, fonts, utility};

/// The sub-nav's width, per the reference.
const SIDEBAR_WIDTH: f32 = 180.0;
/// The sides a window resolution may take, in pixels.
const DIMENSION_RANGE: RangeInclusive<u32> = 320..=7680;

/// The Java runtimes the runtime select offers.
pub const JAVA_RUNTIMES: [&str; 3] = ["Temurin 17.0.9", "Temurin 21.0.2", "System Java 17.0.8"];
/// The garbage collectors the collector select offers.
pub const GARBAGE_COLLECTORS: [&str; 3] = ["G1GC", "ZGC", "Parallel GC"];
/// The Minecraft versions the version select offers.
pub const MINECRAFT_VERSIONS: [&str; 4] = ["1.20.1", "1.20.4", "1.19.4", "1.16.5"];
/// The allocation slider's range, in gigabytes.
pub const MEMORY_RANGE: RangeInclusive<u8> = 2..=16;
/// The machine's installed memory, shown beside the slider.
pub const SYSTEM_MEMORY_GB: u8 = 16;
/// The launch modes the segmented control offers.
pub const LAUNCH_MODES: [(LaunchMode, &str); 3] = [
    (LaunchMode::Windowed, "Windowed"),
    (LaunchMode::Borderless, "Borderless"),
    (LaunchMode::Fullscreen, "Fullscreen"),
];

/// The sections of the sub-nav, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    JavaMemory,
    Window,
    LaunchArgs,
    GameVersion,
    PathsFiles,
    Advanced,
    DangerZone,
}

impl Section {
    pub const ALL: [Section; 7] = [
        Section::JavaMemory,
        Section::Window,
        Section::LaunchArgs,
        Section::GameVersion,
        Section::PathsFiles,
        Section::Advanced,
        Section::DangerZone,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Section::JavaMemory => "Java & Memory",
            Section::Window => "Window",
            Section::LaunchArgs => "Launch Args",
            Section::GameVersion => "Game Version",
            Section::PathsFiles => "Paths & Files",
            Section::Advanced => "Advanced",
            Section::DangerZone => "Danger Zone",
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Section::JavaMemory => "Runtime selection and heap allocation for this instance.",
            Section::Window => "How Minecraft appears when this instance launches.",
            Section::LaunchArgs => "Extra arguments passed to the game when it starts.",
            Section::GameVersion => "Change Minecraft or mod-loader versions. Requires rebuild.",
            Section::PathsFiles => "Where this instance keeps its files.",
            Section::Advanced => "Power-user options. Change with care.",
            Section::DangerZone => "Destructive actions. These cannot be undone.",
        }
    }

    /// The Lucide glyph the section's tile and sub-nav entry show.
    pub fn icon(self) -> &'static str {
        match self {
            Section::JavaMemory => Icon::lucide().cpu(),
            Section::Window => Icon::lucide().image(),
            Section::LaunchArgs => Icon::lucide().terminal(),
            Section::GameVersion => Icon::lucide().tag(),
            Section::PathsFiles => Icon::lucide().folder(),
            Section::Advanced => Icon::lucide().settings(),
            Section::DangerZone => Icon::lucide().trash(),
        }
    }

    /// The accent the section's tile is drawn in.
    pub fn tint(self) -> Color {
        match self {
            Section::DangerZone => color::buttons::Danger::BACKGROUND_HOVER,
            _ => color::Primary::DEFAULT,
        }
    }
}

/// The window's launch mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LaunchMode {
    #[default]
    Windowed,
    Borderless,
    Fullscreen,
}

/// The on/off settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    SystemJava,
    Snapshots,
    HideLauncher,
    QuitAfterExit,
    VerboseLogging,
    SkipIntegrity,
}

/// The settings that pick from a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    JavaRuntime,
    GarbageCollector,
    MinecraftVersion,
    LoaderVersion,
}

/// Which side of the resolution a field edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Width,
    Height,
}

/// One instance's launch configuration. Choices hold an index into the option
/// list they belong to.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Settings {
    pub java_runtime: usize,
    pub garbage_collector: usize,
    pub minecraft_version: usize,
    pub loader_versions: Vec<String>,
    pub loader_version: usize,
    pub memory_gb: u8,
    pub system_java: bool,
    pub snapshots: bool,
    pub launch_mode: LaunchMode,
    pub width: String,
    pub height: String,
    pub hide_launcher: bool,
    pub quit_after_exit: bool,
    pub game_args: String,
    pub environment_variables: usize,
    pub verbose_logging: bool,
    pub skip_integrity: bool,
    /// How many settings are inherited from the global defaults.
    pub inherited: usize,
}

impl Settings {
    /// The launch settings an instance starts with. Nothing persists yet, so
    /// every value is a neutral default. The instance's own Minecraft and loader
    /// versions are the only figures that come from the instance.
    pub fn for_instance(instance: &Instance) -> Self {
        Self {
            minecraft_version: MINECRAFT_VERSIONS.iter().position(|version| *version == instance.mc_version).unwrap_or(0),
            loader_versions: vec![instance.loader_version.clone().unwrap_or_else(|| "—".to_string())],
            memory_gb: 4,
            width: "1920".to_string(),
            height: "1080".to_string(),
            launch_mode: LaunchMode::Windowed,
            ..Self::default()
        }
    }

    fn set_index(&mut self, choice: Choice, index: usize) {
        match choice {
            Choice::JavaRuntime => self.java_runtime = index,
            Choice::GarbageCollector => self.garbage_collector = index,
            Choice::MinecraftVersion => self.minecraft_version = index,
            Choice::LoaderVersion => self.loader_version = index,
        }
    }

    fn switch_mut(&mut self, switch: Switch) -> &mut bool {
        match switch {
            Switch::SystemJava => &mut self.system_java,
            Switch::Snapshots => &mut self.snapshots,
            Switch::HideLauncher => &mut self.hide_launcher,
            Switch::QuitAfterExit => &mut self.quit_after_exit,
            Switch::VerboseLogging => &mut self.verbose_logging,
            Switch::SkipIntegrity => &mut self.skip_integrity,
        }
    }
}

/// Clamps a slider reading to the allocation range, in whole gigabytes.
pub fn clamp_memory(gb: f32) -> u8 {
    gb.round().clamp(f32::from(*MEMORY_RANGE.start()), f32::from(*MEMORY_RANGE.end())) as u8
}

/// Reads a resolution side, if it is a whole number inside the allowed range.
pub fn parse_dimension(text: &str) -> Option<u32> {
    text.parse::<u32>().ok().filter(|value| DIMENSION_RANGE.contains(value))
}

/// Keeps the digits of a resolution field and caps it at five characters.
pub fn sanitize_dimension(text: &str) -> String {
    text.chars().filter(|character| character.is_ascii_digit()).take(5).collect()
}

/// Messages the Settings tab emits.
#[derive(Debug, Clone)]
pub enum Message {
    Section(Section),
    Switch(Switch, bool),
    Choose(Choice, usize),
    /// The allocation slider moved.
    Memory(f32),
    LaunchMode(LaunchMode),
    Dimension(Axis, String),
    GameArgs(String),
    /// The buttons below are display-only until the launch pipeline, file
    /// manager and instance manager land.
    Reinstall,
    ResetSettings,
    DeleteInstance,
    EditEnvironment,
    AddScript,
    RevealFolder,
}

/// The Settings tab's state: the open section and its values.
#[derive(Debug, Default)]
pub struct State {
    section: Section,
    settings: Settings,
}

impl State {
    /// The settings an instance starts with, with the first section open.
    pub fn for_instance(instance: &Instance) -> Self {
        Self {
            section: Section::default(),
            settings: Settings::for_instance(instance),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::Section(section) => self.section = section,
            Message::Switch(switch, on) => *self.settings.switch_mut(switch) = on,
            Message::Choose(choice, index) => self.settings.set_index(choice, index),
            Message::Memory(gb) => self.settings.memory_gb = clamp_memory(gb),
            Message::LaunchMode(mode) => self.settings.launch_mode = mode,
            Message::Dimension(axis, text) => {
                let text = sanitize_dimension(&text);
                match axis {
                    Axis::Width => self.settings.width = text,
                    Axis::Height => self.settings.height = text,
                }
            }
            Message::GameArgs(text) => self.settings.game_args = text,
            Message::Reinstall
            | Message::ResetSettings
            | Message::DeleteInstance
            | Message::EditEnvironment
            | Message::AddScript
            | Message::RevealFolder => {}
        }
    }

    /// The sub-nav beside the open section's header and rows.
    pub fn view<'a>(&'a self, instance: &'a Instance) -> Element<'a, Message> {
        let nav = column(Section::ALL.map(|section| nav_item(section, section == self.section)))
            .spacing(2)
            .width(Fill);
        let mut sidebar = column![nav].spacing(18).width(SIDEBAR_WIDTH);
        // Only shown once there are global defaults to inherit from.
        if self.settings.inherited > 0 {
            sidebar = sidebar.push(inherited_note(self.settings.inherited));
        }
        let body = column![
            section_header(self.section.icon(), self.section.tint(), self.section.label(), self.section.subtitle()),
            sections::body(self.section, &self.settings, instance),
        ]
        .spacing(16)
        .width(Fill);

        row![sidebar, body].spacing(28).align_y(alignment::Vertical::Top).into()
    }
}

/// One sub-nav entry: glyph and label, lit green when its section is open.
fn nav_item(section: Section, active: bool) -> Element<'static, Message> {
    let ink = if active { color::Primary::DEFAULT } else { color::Text::MUTED };
    container(
        TextButton::content(
            row![
                utility::icon(section.icon(), 14.0, Some((ink, ink))),
                Text::new(section.label()).size(13).font(fonts::Inter::MEDIUM).color(ink),
            ]
            .spacing(10)
            .align_y(alignment::Vertical::Center),
        )
        .variant(Variant::Tab)
        .width(Fill)
        .padding(Padding::new(0.0).left(12).right(12).top(8).bottom(8))
        .on_press(Message::Section(section)),
    )
    .width(Fill)
    .style(move |_| container::Style {
        background: active.then_some(Background::Color(color::Primary::TINT)),
        border: Border {
            radius: 10.0.into(),
            ..Border::default()
        },
        ..container::Style::default()
    })
    .into()
}

/// The note under the sub-nav: how many settings come from the global defaults.
fn inherited_note(count: usize) -> Element<'static, Message> {
    container(
        column![
            Text::new("INHERITED")
                .size(10)
                .font(fonts::JetBrainsMono::SEMI_BOLD)
                .color(color::Text::PLACEHOLDER),
            Text::new(format!(
                "{count} settings inherited from Global defaults. Override anything by toggling it here."
            ))
            .size(11)
            .font(fonts::Inter::REGULAR)
            .color(color::Text::MUTED)
            .line_height(1.5),
        ]
        .spacing(8),
    )
    .width(Fill)
    .padding(14)
    .style(|_| container::Style {
        background: Some(Background::Color(rgb!(255, 255, 255, 2%))),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: color::Surface::LINE,
        },
        ..container::Style::default()
    })
    .into()
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/settings/mod.rs"]
mod tests;
