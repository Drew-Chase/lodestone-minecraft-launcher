//! The launch settings as rows. An instance's Settings tab and the global defaults page lay their launch sections out
//! with these, so a setting looks and behaves the same in both. The rows show the values a launch uses, and every
//! change leaves as a [`LaunchChange`] for the view's owner to keep.

use std::ops::RangeInclusive;

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row, slider, text_input};
use iced::{Background, Border, Element, Fill, Padding, alignment};
use iced_color_extension::rgb;

use crate::app::components::button::Variant;
use crate::app::components::modal::Modal;
use crate::app::components::panel::divided_panel;
use crate::app::components::segmented::segmented;
use crate::app::components::setting_row;
use crate::app::components::{Select, TextButton, Toggle, input};
use crate::app::install::catalog::BundledJava;
use crate::app::lang;
use crate::app::launch_settings::{
    Collector, EnvVar, InstanceSettings, JavaChoice, LaunchChange, LaunchMode, LaunchSettings, Resolution, valid_env_name,
};
use crate::app::theme::{color, fonts};

/// The smallest heap the slider offers, in gigabytes.
pub const MIN_MEMORY_GB: u32 = 2;
/// The sides a window resolution may take, in pixels.
const DIMENSION_RANGE: RangeInclusive<u32> = 320..=7680;
/// The collector names the select offers, in the order of [`Collector::ALL`].
const COLLECTOR_LABELS: [&str; 3] = ["G1GC", "ZGC", "Parallel GC"];

/// Horizontal padding inside every section panel.
pub fn panel_padding() -> Padding {
    Padding::new(0.0).left(18.0).right(18.0)
}

/// Which side of the resolution a field edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Width,
    Height,
}

/// The launch settings a row can show. [`Field::inherits`] says which of them an instance has not overridden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Java,
    Memory,
    Collector,
    JvmArgs,
    GameArgs,
    Env,
    Resolution,
    LaunchMode,
    HideLauncher,
    QuitAfterExit,
    VerboseLogging,
    SkipIntegrity,
    PreLaunchHook,
}

/// The Java runtimes the select offers. The bundled runtimes come from the platform's list, and System Java is offered
/// only when a Java was found on `JAVA_HOME` or `PATH`.
#[derive(Debug, Clone, Default)]
pub struct JavaOptions {
    pub bundled: Vec<BundledJava>,
    /// The major version of the system Java, when one was found.
    pub system: Option<u32>,
}

impl JavaOptions {
    /// The choices in select order, each with its label.
    fn choices(&self) -> Vec<(JavaChoice, String)> {
        let mut choices = vec![(JavaChoice::Follow, lang::t("instance.settings.java_follow").to_owned())];
        choices.extend(self.bundled.iter().map(|java| {
            (
                JavaChoice::Bundled {
                    component: java.component.clone(),
                },
                java.label.clone(),
            )
        }));
        if let Some(major) = self.system {
            choices.push((
                JavaChoice::System,
                lang::t_with("instance.settings.java_system", &[("major", major.to_string().as_str())]),
            ));
        }
        choices
    }
}

/// What the rows show: the values a launch uses, which of them the instance overrides, and the lists the selects use.
#[derive(Debug, Clone, Copy)]
pub struct Shown<'a> {
    pub values: &'a LaunchSettings,
    /// The instance's own overrides. `None` on the global defaults page, where nothing inherits.
    pub overrides: Option<&'a InstanceSettings>,
    pub java: &'a JavaOptions,
    /// The largest heap the slider offers, in gigabytes: the machine's memory.
    pub ceiling_gb: u32,
    /// Whether the window is narrow, so each row stacks its control under its label instead of beside it.
    pub narrow: bool,
}

impl Shown<'_> {
    /// Whether `field` takes the global default on this view.
    fn inherits(&self, field: Field) -> bool {
        self.overrides.is_some_and(|overrides| match field {
            Field::Java => overrides.java.is_none(),
            Field::Memory => overrides.memory_mb.is_none(),
            Field::Collector => overrides.collector.is_none(),
            Field::JvmArgs => overrides.jvm_args.is_none(),
            Field::GameArgs => overrides.game_args.is_none(),
            Field::Env => overrides.env.is_none(),
            Field::Resolution => overrides.resolution.is_none(),
            Field::LaunchMode => overrides.launch_mode.is_none(),
            Field::HideLauncher => overrides.hide_launcher.is_none(),
            Field::QuitAfterExit => overrides.quit_after_exit.is_none(),
            Field::VerboseLogging => overrides.verbose_logging.is_none(),
            Field::SkipIntegrity => overrides.skip_integrity.is_none(),
            Field::PreLaunchHook => overrides.pre_launch_hook.is_none(),
        })
    }
}

/// The width and height fields. A field holds the text being typed while the pair is not a valid resolution, and is
/// `None` while it shows the saved value.
#[derive(Debug, Clone, Default)]
pub struct ResolutionDraft {
    pub width: Option<String>,
    pub height: Option<String>,
}

impl ResolutionDraft {
    /// The text each field shows: the draft where there is one, otherwise the saved side.
    pub fn texts(&self, saved: Resolution) -> (String, String) {
        (
            self.width.clone().unwrap_or_else(|| saved.width.to_string()),
            self.height.clone().unwrap_or_else(|| saved.height.to_string()),
        )
    }
}

/// A change to the environment variables. The modal emits these, and the owner applies them with [`EnvEdit::apply`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvEdit {
    Name { index: usize, text: String },
    Value { index: usize, text: String },
    Add,
    Remove(usize),
}

impl EnvEdit {
    /// Applies the edit to `vars`. A row is only added or changed in place, so the list keeps its order.
    pub fn apply(self, vars: &mut Vec<EnvVar>) {
        match self {
            EnvEdit::Name { index, text } => {
                if let Some(var) = vars.get_mut(index) {
                    var.name = text;
                }
            }
            EnvEdit::Value { index, text } => {
                if let Some(var) = vars.get_mut(index) {
                    var.value = text;
                }
            }
            EnvEdit::Add => vars.push(EnvVar::default()),
            EnvEdit::Remove(index) => {
                if index < vars.len() {
                    vars.remove(index);
                }
            }
        }
    }
}

/// A row with its label, an optional hint and its control. A row whose setting is inherited shows an INHERITED tag. On a
/// narrow window the control sits under the label, at its own width, instead of beside it, where it would be squeezed.
fn row_of<'a, M: 'a>(
    narrow: bool,
    name: impl Into<String>,
    hint: Option<String>,
    control: impl Into<Element<'a, M>>,
    inherited: bool,
) -> Element<'a, M> {
    let mut title = row![setting_row::label(name)].spacing(8).align_y(alignment::Vertical::Center);
    if inherited {
        title = title.push(inherited_tag());
    }
    let mut text = column![title].spacing(3);
    if let Some(hint) = hint {
        text = text.push(Text::new(hint).size(11).font(fonts::Inter::REGULAR).color(color::text_muted()));
    }
    if narrow {
        return column![text, control.into()]
            .spacing(10)
            .padding(Padding::new(0.0).top(14).bottom(14))
            .into();
    }
    row![text, Space::new().width(Fill), control.into()]
        .spacing(16)
        .align_y(alignment::Vertical::Center)
        .padding(Padding::new(0.0).top(14).bottom(14))
        .into()
}

/// The small tag beside a label whose setting takes the global default.
fn inherited_tag<'a, M: 'a>() -> Element<'a, M> {
    container(
        Text::new(lang::t("instance.settings.inherited_label"))
            .size(9)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::text_muted()),
    )
    .padding(Padding::new(0.0).left(6).right(6).top(2).bottom(2))
    .style(|_| container::Style {
        background: Some(Background::Color(rgb!(255, 255, 255, 4%))),
        border: Border {
            radius: 6.0.into(),
            width: 1.0,
            color: color::surface_line(),
        },
        ..container::Style::default()
    })
    .into()
}

/// A switch that reports the flipped value through `on`.
fn toggle<M: Clone + 'static>(value: bool, on: impl Fn(LaunchChange) -> M, change: fn(bool) -> LaunchChange) -> Toggle<M> {
    Toggle::new(value).on_toggle(on(change(!value)))
}

/// The width of a resolution side, in pixels, if it is a whole number inside the range the game accepts.
pub fn parse_dimension(text: &str) -> Option<u32> {
    text.parse::<u32>().ok().filter(|value| DIMENSION_RANGE.contains(value))
}

/// Keeps the digits of a resolution field and caps it at five characters.
pub fn sanitize_dimension(text: &str) -> String {
    text.chars().filter(|character| character.is_ascii_digit()).take(5).collect()
}

/// The machine's installed memory in whole gigabytes, read from the OS. It reads the system's memory table, so it runs
/// on the blocking pool.
pub fn machine_memory_gb() -> u32 {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let gb = system.total_memory() / (1024 * 1024 * 1024);
    u32::try_from(gb).unwrap_or(u32::MAX).max(MIN_MEMORY_GB)
}

/// The garbage collector row, which the Java & Memory tab shows under its memory.
pub fn collector_row<'a, M: Clone + 'static>(shown: Shown<'a>, on: impl Fn(LaunchChange) -> M + Clone + 'a) -> Element<'a, M> {
    let collector_index = Collector::ALL.iter().position(|collector| *collector == shown.values.collector);
    let collector = Select::new(COLLECTOR_LABELS, collector_index)
        .on_select(move |index| on(LaunchChange::Collector(Collector::ALL[index])))
        .width(200);
    row_of(
        shown.narrow,
        lang::t("instance.settings.garbage_collector"),
        Some(lang::t("instance.settings.garbage_collector_hint").to_owned()),
        collector,
        shown.inherits(Field::Collector),
    )
}

/// The Java runtime, memory, collector and JVM argument rows.
pub fn java_memory<'a, M: Clone + 'static>(shown: Shown<'a>, on: impl Fn(LaunchChange) -> M + Clone + 'a) -> Element<'a, M> {
    let choices = shown.java.choices();
    let selected = choices.iter().position(|(choice, _)| *choice == shown.values.java);
    let labels: Vec<String> = choices.iter().map(|(_, label)| label.clone()).collect();
    let picks: Vec<JavaChoice> = choices.into_iter().map(|(choice, _)| choice).collect();
    let on_java = on.clone();
    let java = Select::new(labels, selected)
        .placeholder(lang::t("instance.settings.java_unavailable"))
        .on_select(move |index| on_java(LaunchChange::Java(picks[index].clone())))
        .width(240);

    let gb = shown.values.memory_mb / 1024;
    let ceiling = shown.ceiling_gb.max(MIN_MEMORY_GB) as f32;
    let on_memory = on.clone();
    let memory = row![
        slider(MIN_MEMORY_GB as f32..=ceiling, gb as f32, move |value| on_memory(LaunchChange::Memory(
            value.round() as u32 * 1024
        )))
        .step(1.0_f32)
        .style(crate::app::components::slider::style)
        .width(200),
        Text::new(lang::t_with("instance.settings.gigabytes", &[("gb", gb.to_string().as_str())]))
            .size(12)
            .font(fonts::JetBrainsMono::SEMI_BOLD)
            .color(color::primary()),
    ]
    .spacing(14)
    .align_y(alignment::Vertical::Center);
    let allocation = lang::t_with(
        "instance.settings.memory_hint",
        &[("allocated", gb.to_string().as_str()), ("total", shown.ceiling_gb.to_string().as_str())],
    );

    let collector_index = Collector::ALL.iter().position(|collector| *collector == shown.values.collector);
    let on_collector = on.clone();
    let collector = Select::new(COLLECTOR_LABELS, collector_index)
        .on_select(move |index| on_collector(LaunchChange::Collector(Collector::ALL[index])))
        .width(200);

    let on_jvm = on.clone();
    let jvm = text_input(lang::t("instance.settings.jvm_placeholder"), &shown.values.jvm_args)
        .on_input(move |text| on_jvm(LaunchChange::JvmArgs(text)))
        .style(input::style)
        .font(fonts::JetBrainsMono::REGULAR)
        .size(13)
        .padding(Padding::new(0.0).left(12).right(12).top(8).bottom(8))
        .width(360);

    divided_panel(
        vec![
            row_of(
                shown.narrow,
                lang::t("instance.settings.java_runtime"),
                Some(lang::t("instance.settings.java_runtime_hint").to_owned()),
                java,
                shown.inherits(Field::Java),
            ),
            row_of(
                shown.narrow,
                lang::t("instance.settings.allocated_memory"),
                Some(allocation),
                memory,
                shown.inherits(Field::Memory),
            ),
            row_of(
                shown.narrow,
                lang::t("instance.settings.garbage_collector"),
                Some(lang::t("instance.settings.garbage_collector_hint").to_owned()),
                collector,
                shown.inherits(Field::Collector),
            ),
            row_of(
                shown.narrow,
                lang::t("instance.settings.jvm_arguments"),
                Some(lang::t("instance.settings.jvm_arguments_hint").to_owned()),
                jvm,
                shown.inherits(Field::JvmArgs),
            ),
        ],
        panel_padding(),
    )
}

/// The launch mode and the resolution rows. General shows them, and it keeps the window toggles as selects of its own.
pub fn display_rows<'a, M: Clone + 'static>(
    shown: Shown<'a>,
    draft: &ResolutionDraft,
    on: impl Fn(LaunchChange) -> M + Clone + 'a,
    on_resolution: impl Fn(Axis, String) -> M + Clone + 'a,
) -> Vec<Element<'a, M>> {
    let on_mode = on.clone();
    let mode = segmented(
        &[
            (LaunchMode::Windowed, lang::t("instance.settings.launch_mode.windowed")),
            (LaunchMode::Fullscreen, lang::t("instance.settings.launch_mode.fullscreen")),
        ],
        shown.values.launch_mode,
        move |mode| on_mode(LaunchChange::LaunchMode(mode)),
    );

    let (width, height) = draft.texts(shown.values.resolution);
    let valid = parse_dimension(&width).is_some() && parse_dimension(&height).is_some();
    let resolution = row![
        dimension_input(width, valid, on_resolution.clone(), Axis::Width),
        Text::new("×").size(12).font(fonts::Inter::REGULAR).color(color::text_muted()),
        dimension_input(height, valid, on_resolution, Axis::Height),
    ]
    .spacing(10)
    .align_y(alignment::Vertical::Center);

    vec![
        row_of(
            shown.narrow,
            lang::t("instance.settings.launch_mode_label"),
            None,
            mode,
            shown.inherits(Field::LaunchMode),
        ),
        row_of(
            shown.narrow,
            lang::t("instance.settings.resolution"),
            None,
            resolution,
            shown.inherits(Field::Resolution),
        ),
    ]
}

/// The launch mode, resolution, and the two window toggles.
pub fn window<'a, M: Clone + 'static>(
    shown: Shown<'a>,
    draft: &ResolutionDraft,
    on: impl Fn(LaunchChange) -> M + Clone + 'a,
    on_resolution: impl Fn(Axis, String) -> M + Clone + 'a,
) -> Element<'a, M> {
    let hide = shown.values.hide_launcher;
    let quit = shown.values.quit_after_exit;
    let mut rows = display_rows(shown, draft, on.clone(), on_resolution);
    rows.push(row_of(
        shown.narrow,
        lang::t("instance.settings.hide_launcher"),
        Some(lang::t("instance.settings.hide_launcher_hint").to_owned()),
        toggle(hide, on.clone(), LaunchChange::HideLauncher),
        shown.inherits(Field::HideLauncher),
    ));
    rows.push(row_of(
        shown.narrow,
        lang::t("instance.settings.quit_after_exit"),
        None,
        toggle(quit, on, LaunchChange::QuitAfterExit),
        shown.inherits(Field::QuitAfterExit),
    ));
    divided_panel(rows, panel_padding())
}

/// A width or height field: digits only, outlined red while the pair is not a valid resolution.
fn dimension_input<'a, M: Clone + 'static>(value: String, valid: bool, on: impl Fn(Axis, String) -> M + 'a, axis: Axis) -> Element<'a, M> {
    text_input("", &value)
        .on_input(move |text| on(axis, text))
        .style(move |theme, status| {
            let mut style = input::style(theme, status);
            if !valid {
                style.border.color = color::buttons::danger_background_hover();
            }
            style
        })
        .font(fonts::JetBrainsMono::REGULAR)
        .size(13)
        .padding(Padding::new(0.0).left(12).right(12).top(8).bottom(8))
        .width(96)
        .into()
}

/// The game-arguments row, in a panel of its own.
pub fn launch_args<'a, M: Clone + 'static>(shown: Shown<'a>, on: impl Fn(LaunchChange) -> M + Clone + 'a) -> Element<'a, M> {
    divided_panel(launch_arg_rows(shown, on), panel_padding())
}

/// The game-arguments row.
pub fn launch_arg_rows<'a, M: Clone + 'static>(shown: Shown<'a>, on: impl Fn(LaunchChange) -> M + Clone + 'a) -> Vec<Element<'a, M>> {
    let arguments = text_input(lang::t("instance.settings.game_arguments_placeholder"), &shown.values.game_args)
        .on_input(move |text| on(LaunchChange::GameArgs(text)))
        .style(input::style)
        .font(fonts::JetBrainsMono::REGULAR)
        .size(13)
        .padding(Padding::new(0.0).left(12).right(12).top(8).bottom(8))
        .width(360);
    vec![row_of(
        shown.narrow,
        lang::t("instance.settings.game_arguments"),
        Some(lang::t("instance.settings.game_arguments_hint").to_owned()),
        arguments,
        shown.inherits(Field::GameArgs),
    )]
}

/// The environment, pre-launch hook, logging and integrity rows, in a panel of their own. The environment opens its
/// editor with `open_env`.
pub fn advanced<'a, M: Clone + 'static>(shown: Shown<'a>, on: impl Fn(LaunchChange) -> M + Clone + 'a, open_env: M) -> Element<'a, M> {
    divided_panel(advanced_rows(shown, on, open_env), panel_padding())
}

/// The environment, pre-launch hook, logging and integrity rows. The environment opens its editor with `open_env`.
pub fn advanced_rows<'a, M: Clone + 'static>(shown: Shown<'a>, on: impl Fn(LaunchChange) -> M + Clone + 'a, open_env: M) -> Vec<Element<'a, M>> {
    let count = shown.values.env.len().to_string();
    let environment = TextButton::ghost(lang::t_with("instance.settings.env_set", &[("count", count.as_str())])).on_press(open_env);
    let on_hook = on.clone();
    let hook = text_input(lang::t("instance.settings.pre_launch_hook_placeholder"), &shown.values.pre_launch_hook)
        .on_input(move |text| on_hook(LaunchChange::PreLaunchHook(text)))
        .style(input::style)
        .font(fonts::JetBrainsMono::REGULAR)
        .size(13)
        .padding(Padding::new(0.0).left(12).right(12).top(8).bottom(8))
        .width(360);
    let verbose = shown.values.verbose_logging;
    let integrity = shown.values.skip_integrity;
    vec![
        row_of(
            shown.narrow,
            lang::t("instance.settings.env_vars"),
            Some(lang::t("instance.settings.env_vars_hint").to_owned()),
            environment,
            shown.inherits(Field::Env),
        ),
        row_of(
            shown.narrow,
            lang::t("instance.settings.pre_launch_hook"),
            Some(lang::t("instance.settings.pre_launch_hook_hint").to_owned()),
            hook,
            shown.inherits(Field::PreLaunchHook),
        ),
        row_of(
            shown.narrow,
            lang::t("instance.settings.verbose_logging"),
            Some(lang::t("instance.settings.verbose_logging_hint").to_owned()),
            toggle(verbose, on.clone(), LaunchChange::VerboseLogging),
            shown.inherits(Field::VerboseLogging),
        ),
        row_of(
            shown.narrow,
            lang::t("instance.settings.skip_integrity"),
            Some(lang::t("instance.settings.skip_integrity_hint").to_owned()),
            toggle(integrity, on, LaunchChange::SkipIntegrity),
            shown.inherits(Field::SkipIntegrity),
        ),
    ]
}

/// The environment variables editor: one row per variable, a name and a value, with a remove button, and an Add
/// button under them. `on_edit` receives each change and `close` shuts the modal.
pub fn environment_modal<'a, M: Clone + 'static>(vars: &'a [EnvVar], on_edit: impl Fn(EnvEdit) -> M + Clone + 'a, close: M) -> Element<'a, M> {
    let mut rows = column![].spacing(8);
    for (index, var) in vars.iter().enumerate() {
        let on_name = on_edit.clone();
        let on_value = on_edit.clone();
        let on_remove = on_edit.clone();
        let name_valid = valid_env_name(&var.name);
        let name = text_input(lang::t("instance.settings.env_name_placeholder"), &var.name)
            .on_input(move |text| on_name(EnvEdit::Name { index, text }))
            .style(move |theme, status| {
                let mut style = input::style(theme, status);
                if !name_valid {
                    style.border.color = color::buttons::danger_background_hover();
                }
                style
            })
            .font(fonts::JetBrainsMono::REGULAR)
            .size(13)
            .padding(Padding::new(0.0).left(12).right(12).top(8).bottom(8))
            .width(200);
        let value = text_input(lang::t("instance.settings.env_value_placeholder"), &var.value)
            .on_input(move |text| on_value(EnvEdit::Value { index, text }))
            .style(input::style)
            .font(fonts::JetBrainsMono::REGULAR)
            .size(13)
            .padding(Padding::new(0.0).left(12).right(12).top(8).bottom(8))
            .width(Fill);
        let remove = TextButton::ghost(lang::t("instance.settings.env_remove")).on_press(on_remove(EnvEdit::Remove(index)));
        rows = rows.push(row![name, value, remove].spacing(10).align_y(alignment::Vertical::Center));
    }
    let add = TextButton::labeled_icon(Icon::lucide().plus(), lang::t("instance.settings.env_add"), Variant::Primary).on_press(on_edit(EnvEdit::Add));
    let body = column![rows, add].spacing(14);
    let footer: Element<'a, M> = TextButton::ghost(lang::t("common.done")).on_press(close.clone()).into();
    Modal::new(lang::t("instance.settings.env_title"), body)
        .subtitle(lang::t("instance.settings.env_hint"))
        .footer(footer)
        .on_close(close)
        .width(640.0)
        .into()
}
