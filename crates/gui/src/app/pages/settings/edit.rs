//! The changes the Settings tabs make to the draft's values. A launch default changes through [`LaunchChange`] instead.

use crate::app::preferences::{DownloadMirror, HeroRule, SettingsValues, SourceOrder, StartupPage};
use crate::app::theme::{Accent, SurfaceTheme};

/// One change to a setting. The tabs emit these, and the draft writes each one into its copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    /// The page the launcher opens on.
    Startup(StartupPage),
    /// Which instance the library's hero banner features.
    Hero(HeroRule),
    /// Installs stable releases of the launcher in the background.
    AutoUpdate(bool),
    /// Offers prereleases of the launcher as well as stable releases.
    BetaChannel(bool),
    /// Refreshes the Minecraft version list in the background.
    AutoUpdateGameVersions(bool),
    /// How many files an install fetches at once.
    ConcurrentDownloads(u16),
    /// The accent the interface highlights with.
    Accent(Accent),
    /// The surface theme: the canvas, the panels and the hairlines.
    Theme(SurfaceTheme),
    /// Card hovers, page transitions and other micro-interactions.
    Animations(bool),
    /// The floating embers on the login screen.
    Particles(bool),
    /// The soft shadows under panels, cards and popovers.
    Glass(bool),
    /// The soft color glows behind the login screen.
    Aurora(bool),
    /// Stops the looping motion and makes transitions instant.
    ReduceMotion(bool),
    /// Moves the navigation into a bar that floats at the bottom of the window.
    FloatingBottomBar(bool),
    /// How many idle connections a host keeps open.
    ConnectionsPerHost(u16),
    /// Which platform a file is looked up on first.
    SourceOrder(SourceOrder),
    /// Where downloads come from.
    Mirror(DownloadMirror),
    /// Seconds a request may take before it fails.
    TimeoutSecs(u32),
    /// Whether the operating system's proxy is followed.
    UseSystemProxy(bool),
    /// Whether the custom proxy is used.
    CustomProxyEnabled(bool),
    /// The custom proxy's address.
    CustomProxy(String),
    /// Whether crash reports are sent.
    CrashReports(bool),
    /// Whether anonymous feature counters are sent.
    UsageStats(bool),
    /// Whether performance diagnostics are recorded for each instance.
    PerformanceDiagnostics(bool),
    /// Whether mods may read and write files.
    FilesystemAccess(bool),
    /// Whether mods may make network requests.
    NetworkAccess(bool),
    /// Whether mods may use input devices and other hardware.
    HardwareAccess(bool),
}

impl Edit {
    /// Writes the change into `values`.
    pub fn apply(self, values: &mut SettingsValues) {
        match self {
            Edit::Startup(page) => values.startup_page = page,
            Edit::Hero(rule) => values.hero = rule,
            Edit::AutoUpdate(on) => values.auto_update = on,
            Edit::BetaChannel(on) => values.beta_channel = on,
            Edit::AutoUpdateGameVersions(on) => values.auto_update_game_versions = on,
            Edit::ConcurrentDownloads(count) => values.network.concurrent_downloads = count,
            Edit::Accent(accent) => values.appearance.accent = accent,
            Edit::Theme(theme) => values.appearance.theme = theme,
            Edit::Animations(on) => values.appearance.interface_animations = on,
            Edit::Particles(on) => values.appearance.ambient_particles = on,
            Edit::Glass(on) => values.appearance.glass_effect = on,
            Edit::Aurora(on) => values.appearance.aurora_backgrounds = on,
            Edit::ReduceMotion(on) => values.appearance.reduce_motion = on,
            Edit::FloatingBottomBar(on) => values.appearance.floating_bottom_bar = on,
            Edit::ConnectionsPerHost(count) => values.network.connections_per_host = count,
            Edit::SourceOrder(order) => values.network.source_order = order,
            Edit::Mirror(mirror) => values.network.mirror = mirror,
            Edit::TimeoutSecs(seconds) => values.network.timeout_secs = seconds,
            Edit::UseSystemProxy(on) => values.network.use_system_proxy = on,
            Edit::CustomProxyEnabled(on) => values.network.custom_proxy_enabled = on,
            Edit::CustomProxy(address) => values.network.custom_proxy = address,
            Edit::CrashReports(on) => values.privacy.crash_reports = on,
            Edit::UsageStats(on) => values.privacy.usage_stats = on,
            Edit::PerformanceDiagnostics(on) => values.privacy.performance_diagnostics = on,
            Edit::FilesystemAccess(on) => values.privacy.filesystem_access = on,
            Edit::NetworkAccess(on) => values.privacy.network_access = on,
            Edit::HardwareAccess(on) => values.privacy.hardware_access = on,
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/settings/edit.rs"]
mod tests;
