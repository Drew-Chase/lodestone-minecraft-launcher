//! Launcher preferences: the choices the user makes that outlive a session.
//!
//! Kept as JSON at `<config_dir>/lodestone/preferences.json`, next to the
//! credential file. `boot` loads it before the first view, so the saved language
//! is already active on the first frame.
//!
//! The Settings page's values sit at the top level of the file, beside the
//! language and the Discover platform. A file written before a value existed
//! gives that value its default.

use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::app::launch_settings::LaunchDefaults;
use crate::app::theme::{Accent, SurfaceTheme};

/// The saved choices. A field that is missing from the file takes its default.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    /// The language's catalogue code, such as `en_us`. `None` means the default.
    pub language: Option<String>,
    /// The content platform the Discover page opened on last, by its key such as `modrinth`.
    pub discover_source: Option<String>,
    /// The folder each launcher was located in by hand, keyed by the launcher's key such as `prism`. The Import dialog
    /// scans these before the standard folders.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub launcher_folders: std::collections::BTreeMap<String, PathBuf>,
    /// The values the Settings page saves. They are flattened into the file's top level.
    #[serde(flatten)]
    pub settings: SettingsValues,
}

impl Preferences {
    /// Loads the saved preferences. A missing, unreadable, or corrupt file yields
    /// the defaults, so a broken file never blocks the launcher.
    pub fn load() -> Self {
        Self::load_from(&preferences_path())
    }

    /// Persists the preferences, creating the directory on first write.
    pub fn store(&self) -> Result<(), String> {
        Self::store_to(&preferences_path(), self)
    }

    /// Loads from an explicit path (the entry point for tests).
    pub fn load_from(path: &Path) -> Self {
        let Ok(contents) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        match serde_json::from_str(&contents) {
            Ok(preferences) => preferences,
            Err(error) => {
                tracing::warn!("preferences file is corrupt, ignoring it: {error}");
                Self::default()
            }
        }
    }

    /// Persists to an explicit path (the entry point for tests).
    pub fn store_to(path: &Path, preferences: &Self) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| format!("creating {}: {error}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(preferences).map_err(|error| format!("serializing preferences: {error}"))?;
        std::fs::write(path, json).map_err(|error| format!("writing {}: {error}", path.display()))
    }

    /// Removes the saved preferences, so the next launch uses the defaults. A file that is already gone is not an error.
    pub fn remove() -> Result<(), String> {
        Self::remove_from(&preferences_path())
    }

    /// Removes the file at an explicit path (the entry point for tests).
    pub fn remove_from(path: &Path) -> Result<(), String> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("removing {}: {error}", path.display())),
        }
    }
}

/// The values the Settings page edits and saves together.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SettingsValues {
    /// The launch settings every instance inherits until it overrides one.
    pub instance_defaults: LaunchDefaults,
    /// The page the launcher opens on.
    pub startup_page: StartupPage,
    /// Installs stable releases of the launcher in the background.
    pub auto_update: bool,
    /// Offers prereleases of the launcher as well as stable releases.
    pub beta_channel: bool,
    /// Refreshes the Minecraft version list in the background at launch.
    pub auto_update_game_versions: bool,
    /// How many files an instance fetches at once. See [`CONCURRENT_DOWNLOADS`].
    pub concurrent_downloads: u16,
    pub appearance: Appearance,
    pub network: Network,
    pub privacy: Privacy,
}

/// The most files an install fetches at once. It is the cap the downloads always used, and so the default.
pub const MAX_CONCURRENT_DOWNLOADS: u16 = 64;

/// The range the concurrent-downloads setting takes.
pub const CONCURRENT_DOWNLOADS: RangeInclusive<u16> = 1..=MAX_CONCURRENT_DOWNLOADS;

impl Default for SettingsValues {
    fn default() -> Self {
        Self {
            instance_defaults: LaunchDefaults::default(),
            startup_page: StartupPage::default(),
            auto_update: true,
            beta_channel: false,
            auto_update_game_versions: true,
            concurrent_downloads: *CONCURRENT_DOWNLOADS.end(),
            appearance: Appearance::default(),
            network: Network::default(),
            privacy: Privacy::default(),
        }
    }
}

impl SettingsValues {
    /// Makes the values live: the palette every view draws with, and how many files a download fetches at once.
    pub fn apply(&self) {
        crate::app::theme::color::set(crate::app::theme::color::Palette::new(&self.appearance.accent, self.appearance.theme));
        crate::app::theme::effects::set(&self.appearance);
        lodestone_net::set(self.network.config());
        crate::app::install::download::set_concurrency(self.concurrent_downloads);
        lodestone_loaders::set_download_concurrency(usize::from(self.concurrent_downloads));
    }
}

/// The page the launcher opens on once someone is signed in.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StartupPage {
    /// The instance library, the launcher's home page.
    #[default]
    Library,
    /// The Discover page.
    Discover,
    /// The Worlds page.
    Worlds,
}

/// How the launcher looks. The accent and the surface theme recolor the whole interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Appearance {
    pub accent: Accent,
    pub theme: SurfaceTheme,
    /// Card hovers, page transitions and other micro-interactions.
    pub interface_animations: bool,
    /// The floating embers on the login screen.
    pub ambient_particles: bool,
    /// The frosted blur behind panels, cards and popovers.
    pub glass_effect: bool,
    /// The soft color glows behind the login screen.
    pub aurora_backgrounds: bool,
    /// Stops the looping motion and makes transitions instant.
    pub reduce_motion: bool,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            accent: Accent::Green,
            theme: SurfaceTheme::Void,
            interface_animations: true,
            ambient_particles: true,
            glass_effect: true,
            aurora_backgrounds: true,
            reduce_motion: false,
        }
    }
}

/// Where the launcher's requests go, and how long they may take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Network {
    /// How many connections a single host may hold open at once.
    pub connections_per_host: u16,
    /// Which platform a file is looked up on first.
    pub source_order: SourceOrder,
    /// Where downloads come from.
    pub mirror: DownloadMirror,
    /// Seconds a request may take before it fails.
    pub timeout_secs: u32,
    /// Follows the operating system's proxy setting.
    pub use_system_proxy: bool,
    /// Sends requests through `custom_proxy` instead of the system proxy.
    pub custom_proxy_enabled: bool,
    /// The proxy's address, such as `proxy.example.com:3128`.
    pub custom_proxy: String,
}

impl Default for Network {
    fn default() -> Self {
        Self {
            connections_per_host: 8,
            source_order: SourceOrder::default(),
            mirror: DownloadMirror::default(),
            timeout_secs: 30,
            use_system_proxy: true,
            custom_proxy_enabled: false,
            custom_proxy: String::new(),
        }
    }
}

impl Network {
    /// The client settings these choices make. A custom proxy with no address is not used.
    pub fn config(&self) -> lodestone_net::Config {
        let address = self.custom_proxy.trim();
        lodestone_net::Config {
            use_system_proxy: self.use_system_proxy,
            custom_proxy: (self.custom_proxy_enabled && !address.is_empty()).then(|| address.to_owned()),
            timeout: std::time::Duration::from_secs(u64::from(self.timeout_secs)),
            idle_per_host: usize::from(self.connections_per_host),
        }
    }
}

/// The platform a file is looked up on first. The other one is the fallback.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceOrder {
    /// Modrinth first, then CurseForge.
    #[default]
    ModrinthFirst,
    /// CurseForge first, then Modrinth.
    CurseforgeFirst,
}

/// Where downloads come from. Mojang's own servers are the only source so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadMirror {
    /// Mojang's servers, the closest of which the launcher's requests reach.
    #[default]
    Auto,
}

/// What the launcher shares, and what mods may reach. The launcher sends no telemetry, and mods have no permission
/// system yet, so these choices are saved but not enforced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Privacy {
    pub crash_reports: bool,
    pub usage_stats: bool,
    pub performance_diagnostics: bool,
    pub filesystem_access: bool,
    pub network_access: bool,
    pub hardware_access: bool,
}

impl Default for Privacy {
    fn default() -> Self {
        Self {
            crash_reports: true,
            usage_stats: true,
            performance_diagnostics: false,
            filesystem_access: true,
            network_access: true,
            hardware_access: false,
        }
    }
}

/// `<config_dir>/lodestone/preferences.json`.
fn preferences_path() -> PathBuf {
    let mut path = crate::app::paths::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("preferences.json");
    path
}

#[cfg(test)]
#[path = "../../tests/app/preferences.rs"]
mod tests;
