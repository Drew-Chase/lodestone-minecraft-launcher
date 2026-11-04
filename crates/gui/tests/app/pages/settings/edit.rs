use super::*;

#[test]
fn each_edit_writes_its_own_value() {
    let mut values = SettingsValues::default();
    Edit::Startup(StartupPage::Worlds).apply(&mut values);
    Edit::AutoUpdate(false).apply(&mut values);
    Edit::BetaChannel(true).apply(&mut values);
    Edit::AutoUpdateGameVersions(false).apply(&mut values);
    Edit::ConcurrentDownloads(11).apply(&mut values);
    assert_eq!(values.startup_page, StartupPage::Worlds);
    assert!(!values.auto_update);
    assert!(values.beta_channel);
    assert!(!values.auto_update_game_versions);
    assert_eq!(values.network.concurrent_downloads, 11);
}

#[test]
fn an_edit_leaves_the_other_values_alone() {
    let mut values = SettingsValues::default();
    Edit::BetaChannel(true).apply(&mut values);
    assert!(values.auto_update);
    assert_eq!(values.network.concurrent_downloads, 64);
}

#[test]
fn the_appearance_edits_write_their_own_fields() {
    let mut values = SettingsValues::default();
    Edit::Accent(Accent::Cyan).apply(&mut values);
    Edit::Theme(SurfaceTheme::Obsidian).apply(&mut values);
    Edit::Animations(false).apply(&mut values);
    Edit::Particles(false).apply(&mut values);
    Edit::Glass(false).apply(&mut values);
    Edit::Aurora(false).apply(&mut values);
    Edit::ReduceMotion(true).apply(&mut values);
    assert_eq!(values.appearance.accent, Accent::Cyan);
    assert_eq!(values.appearance.theme, SurfaceTheme::Obsidian);
    assert!(!values.appearance.interface_animations);
    assert!(!values.appearance.ambient_particles);
    assert!(!values.appearance.glass_effect);
    assert!(!values.appearance.aurora_backgrounds);
    assert!(values.appearance.reduce_motion);
}

#[test]
fn the_floating_bar_edit_writes_its_own_field() {
    let mut values = SettingsValues::default();
    assert!(!values.appearance.floating_bottom_bar);
    Edit::FloatingBottomBar(true).apply(&mut values);
    assert!(values.appearance.floating_bottom_bar);
    assert!(!values.appearance.reduce_motion);
}

#[test]
fn the_privacy_edits_write_their_own_fields() {
    let mut values = SettingsValues::default();
    Edit::CrashReports(false).apply(&mut values);
    Edit::UsageStats(false).apply(&mut values);
    Edit::PerformanceDiagnostics(true).apply(&mut values);
    Edit::FilesystemAccess(false).apply(&mut values);
    Edit::NetworkAccess(false).apply(&mut values);
    Edit::HardwareAccess(true).apply(&mut values);
    assert!(!values.privacy.crash_reports);
    assert!(!values.privacy.usage_stats);
    assert!(values.privacy.performance_diagnostics);
    assert!(!values.privacy.filesystem_access);
    assert!(!values.privacy.network_access);
    assert!(values.privacy.hardware_access);
}
