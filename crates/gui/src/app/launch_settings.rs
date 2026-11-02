//! The launch settings: the values a game starts with, the global defaults they come from, and the
//! overrides an instance keeps for itself.
//!
//! The global defaults live in the preferences, and every value there is set. An instance keeps an
//! `Option` for each setting, where `None` inherits the default. A launch reads [`LaunchSettings`],
//! which is the two merged. The overrides are stored as JSON on the instance's row, so a new setting
//! needs no migration.

use serde::{Deserialize, Serialize};

/// The heap a game gets when nothing says otherwise, in megabytes.
pub const DEFAULT_MEMORY_MB: u32 = 4096;

/// Which Java runtime a launch runs the game with.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JavaChoice {
    /// The runtime the Minecraft version asks for.
    #[default]
    Follow,
    /// A bundled Mojang runtime, by its component such as `java-runtime-delta`.
    Bundled { component: String },
    /// A Java found through `JAVA_HOME` or `PATH`.
    System,
}

/// The garbage collector a launch selects.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Collector {
    #[default]
    G1,
    Zgc,
    Parallel,
}

impl Collector {
    /// Every collector, in the order the select offers them.
    pub const ALL: [Collector; 3] = [Collector::G1, Collector::Zgc, Collector::Parallel];

    /// The JVM flag that selects this collector.
    pub fn flag(self) -> &'static str {
        match self {
            Collector::G1 => "-XX:+UseG1GC",
            Collector::Zgc => "-XX:+UseZGC",
            Collector::Parallel => "-XX:+UseParallelGC",
        }
    }
}

/// Whether the game opens in a window or fills the screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LaunchMode {
    #[default]
    Windowed,
    Fullscreen,
}

/// The window's size in pixels, passed to the game as its width and height.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Default for Resolution {
    fn default() -> Self {
        Self { width: 1920, height: 1080 }
    }
}

/// One environment variable the game process gets.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
}

/// A change to one launch setting. The settings views emit these, and whichever owner holds the value
/// takes it: the global defaults, or an instance's overrides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchChange {
    Java(JavaChoice),
    Memory(u32),
    Collector(Collector),
    JvmArgs(String),
    GameArgs(String),
    Env(Vec<EnvVar>),
    Resolution(Resolution),
    LaunchMode(LaunchMode),
    HideLauncher(bool),
    QuitAfterExit(bool),
    VerboseLogging(bool),
    SkipIntegrity(bool),
    PreLaunchHook(String),
}

/// The global defaults for the launch settings. Every value is set; [`Default`] holds the built-in values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LaunchDefaults {
    pub java: JavaChoice,
    /// The heap the game gets, in megabytes.
    pub memory_mb: u32,
    pub collector: Collector,
    pub jvm_args: String,
    pub game_args: String,
    pub env: Vec<EnvVar>,
    pub resolution: Resolution,
    pub launch_mode: LaunchMode,
    pub hide_launcher: bool,
    pub quit_after_exit: bool,
    pub verbose_logging: bool,
    pub skip_integrity: bool,
    pub pre_launch_hook: String,
}

impl Default for LaunchDefaults {
    fn default() -> Self {
        Self {
            java: JavaChoice::Follow,
            memory_mb: DEFAULT_MEMORY_MB,
            collector: Collector::G1,
            jvm_args: String::new(),
            game_args: String::new(),
            env: Vec::new(),
            resolution: Resolution::default(),
            launch_mode: LaunchMode::Windowed,
            hide_launcher: false,
            quit_after_exit: false,
            verbose_logging: false,
            skip_integrity: false,
            pre_launch_hook: String::new(),
        }
    }
}

impl LaunchDefaults {
    /// Takes a change to a default.
    pub fn apply(&mut self, change: LaunchChange) {
        match change {
            LaunchChange::Java(value) => self.java = value,
            LaunchChange::Memory(value) => self.memory_mb = value,
            LaunchChange::Collector(value) => self.collector = value,
            LaunchChange::JvmArgs(value) => self.jvm_args = value,
            LaunchChange::GameArgs(value) => self.game_args = value,
            LaunchChange::Env(value) => self.env = value,
            LaunchChange::Resolution(value) => self.resolution = value,
            LaunchChange::LaunchMode(value) => self.launch_mode = value,
            LaunchChange::HideLauncher(value) => self.hide_launcher = value,
            LaunchChange::QuitAfterExit(value) => self.quit_after_exit = value,
            LaunchChange::VerboseLogging(value) => self.verbose_logging = value,
            LaunchChange::SkipIntegrity(value) => self.skip_integrity = value,
            LaunchChange::PreLaunchHook(value) => self.pre_launch_hook = value,
        }
    }
}

/// An instance's own values. `None` inherits the global default for that setting.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct InstanceSettings {
    pub java: Option<JavaChoice>,
    pub memory_mb: Option<u32>,
    pub collector: Option<Collector>,
    pub jvm_args: Option<String>,
    pub game_args: Option<String>,
    pub env: Option<Vec<EnvVar>>,
    pub resolution: Option<Resolution>,
    pub launch_mode: Option<LaunchMode>,
    pub hide_launcher: Option<bool>,
    pub quit_after_exit: Option<bool>,
    pub verbose_logging: Option<bool>,
    pub skip_integrity: Option<bool>,
    pub pre_launch_hook: Option<String>,
    /// Whether the Game Version section lists snapshots. Not inherited, and kept by a reset.
    pub snapshots: bool,
}

impl InstanceSettings {
    /// Overrides the default for the setting `change` names.
    pub fn override_with(&mut self, change: LaunchChange) {
        match change {
            LaunchChange::Java(value) => self.java = Some(value),
            LaunchChange::Memory(value) => self.memory_mb = Some(value),
            LaunchChange::Collector(value) => self.collector = Some(value),
            LaunchChange::JvmArgs(value) => self.jvm_args = Some(value),
            LaunchChange::GameArgs(value) => self.game_args = Some(value),
            LaunchChange::Env(value) => self.env = Some(value),
            LaunchChange::Resolution(value) => self.resolution = Some(value),
            LaunchChange::LaunchMode(value) => self.launch_mode = Some(value),
            LaunchChange::HideLauncher(value) => self.hide_launcher = Some(value),
            LaunchChange::QuitAfterExit(value) => self.quit_after_exit = Some(value),
            LaunchChange::VerboseLogging(value) => self.verbose_logging = Some(value),
            LaunchChange::SkipIntegrity(value) => self.skip_integrity = Some(value),
            LaunchChange::PreLaunchHook(value) => self.pre_launch_hook = Some(value),
        }
    }

    /// How many launch settings take the global default.
    pub fn inherited_count(&self) -> usize {
        [
            self.java.is_none(),
            self.memory_mb.is_none(),
            self.collector.is_none(),
            self.jvm_args.is_none(),
            self.game_args.is_none(),
            self.env.is_none(),
            self.resolution.is_none(),
            self.launch_mode.is_none(),
            self.hide_launcher.is_none(),
            self.quit_after_exit.is_none(),
            self.verbose_logging.is_none(),
            self.skip_integrity.is_none(),
            self.pre_launch_hook.is_none(),
        ]
        .into_iter()
        .filter(|inherited| *inherited)
        .count()
    }

    /// Drops every launch override, so each setting inherits again. Versions and snapshots are kept.
    pub fn reset(&mut self) {
        *self = Self {
            snapshots: self.snapshots,
            ..Self::default()
        };
    }

    /// The values a launch uses: each override, or the default where there is none.
    pub fn resolve(&self, defaults: &LaunchDefaults) -> LaunchSettings {
        LaunchSettings {
            java: or(&self.java, &defaults.java),
            memory_mb: or(&self.memory_mb, &defaults.memory_mb),
            collector: or(&self.collector, &defaults.collector),
            jvm_args: or(&self.jvm_args, &defaults.jvm_args),
            game_args: or(&self.game_args, &defaults.game_args),
            env: or(&self.env, &defaults.env),
            resolution: or(&self.resolution, &defaults.resolution),
            launch_mode: or(&self.launch_mode, &defaults.launch_mode),
            hide_launcher: or(&self.hide_launcher, &defaults.hide_launcher),
            quit_after_exit: or(&self.quit_after_exit, &defaults.quit_after_exit),
            verbose_logging: or(&self.verbose_logging, &defaults.verbose_logging),
            skip_integrity: or(&self.skip_integrity, &defaults.skip_integrity),
            pre_launch_hook: or(&self.pre_launch_hook, &defaults.pre_launch_hook),
        }
    }
}

/// The override when there is one, otherwise the default.
fn or<T: Clone>(setting: &Option<T>, default: &T) -> T {
    setting.clone().unwrap_or_else(|| default.clone())
}

/// The values one launch uses, after the instance's overrides and the defaults are merged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSettings {
    pub java: JavaChoice,
    pub memory_mb: u32,
    pub collector: Collector,
    pub jvm_args: String,
    pub game_args: String,
    pub env: Vec<EnvVar>,
    pub resolution: Resolution,
    pub launch_mode: LaunchMode,
    pub hide_launcher: bool,
    pub quit_after_exit: bool,
    pub verbose_logging: bool,
    pub skip_integrity: bool,
    pub pre_launch_hook: String,
}

/// Splits `text` into arguments. Whitespace separates them, and double quotes keep a run of words
/// together and are dropped, so `-Dname="a b"` is one argument.
pub fn split_arguments(text: &str) -> Vec<String> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut started = false;
    for character in text.chars() {
        match character {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            whitespace if whitespace.is_whitespace() && !quoted => {
                if started {
                    arguments.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            other => {
                current.push(other);
                started = true;
            }
        }
    }
    if started {
        arguments.push(current);
    }
    arguments
}

/// Whether `name` can name an environment variable: not empty, and free of `=` and NUL.
pub fn valid_env_name(name: &str) -> bool {
    !name.is_empty() && !name.chars().any(|character| character == '=' || character == '\0')
}

#[cfg(test)]
#[path = "../../tests/app/launch_settings.rs"]
mod tests;
