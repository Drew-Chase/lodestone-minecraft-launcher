//! The Privacy tab: the telemetry and mod permissions the launcher records, which are saved but not enforced yet, and the
//! actions on the data the launcher keeps on this machine.

use aurora_leaf::Icon;
use iced::widget::{column, row, text};
use iced::{Element, Fill};

use super::edit::Edit;
use super::{Message, card};
use crate::app::components::button::Variant;
use crate::app::components::panel::divided;
use crate::app::components::setting_row::described_setting;
use crate::app::components::{TextButton, Toggle};
use crate::app::lang;
use crate::app::preferences::{Privacy, SettingsValues};
use crate::app::theme::{color, fonts};

/// The Privacy tab: the telemetry switches, the mod permission switches, then the data actions.
pub fn view<'a>(settings: &SettingsValues) -> Element<'a, Message> {
    column![
        card(
            lang::t("settings.privacy.telemetry_title"),
            lang::t("settings.privacy.telemetry_subtitle"),
            column![not_enforced(), divided(telemetry_rows(&settings.privacy))].spacing(16).into(),
        ),
        card(
            lang::t("settings.privacy.permissions_title"),
            lang::t("settings.privacy.permissions_subtitle"),
            column![not_enforced(), divided(permission_rows(&settings.privacy))].spacing(16).into(),
        ),
        card(
            lang::t("settings.privacy.data_title"),
            lang::t("settings.privacy.data_subtitle"),
            data_actions(),
        ),
    ]
    .spacing(24)
    .width(Fill)
    .into()
}

/// The telemetry switches: crash reports, anonymous usage counters and performance diagnostics.
fn telemetry_rows<'a>(privacy: &Privacy) -> Vec<Element<'a, Message>> {
    vec![
        switch_row(
            lang::t("settings.privacy.crash_reports"),
            lang::t("settings.privacy.crash_reports_hint"),
            privacy.crash_reports,
            Edit::CrashReports,
        ),
        switch_row(
            lang::t("settings.privacy.usage_stats"),
            lang::t("settings.privacy.usage_stats_hint"),
            privacy.usage_stats,
            Edit::UsageStats,
        ),
        switch_row(
            lang::t("settings.privacy.performance"),
            lang::t("settings.privacy.performance_hint"),
            privacy.performance_diagnostics,
            Edit::PerformanceDiagnostics,
        ),
    ]
}

/// The mod permission switches: filesystem, network and hardware access.
fn permission_rows<'a>(privacy: &Privacy) -> Vec<Element<'a, Message>> {
    vec![
        switch_row(
            lang::t("settings.privacy.filesystem"),
            lang::t("settings.privacy.filesystem_hint"),
            privacy.filesystem_access,
            Edit::FilesystemAccess,
        ),
        switch_row(
            lang::t("settings.privacy.network"),
            lang::t("settings.privacy.network_hint"),
            privacy.network_access,
            Edit::NetworkAccess,
        ),
        switch_row(
            lang::t("settings.privacy.hardware"),
            lang::t("settings.privacy.hardware_hint"),
            privacy.hardware_access,
            Edit::HardwareAccess,
        ),
    ]
}

/// A row with a switch that shows `on`. Pressing the switch sends `edit` with the opposite value.
fn switch_row<'a>(name: &'static str, hint: &'static str, on: bool, edit: fn(bool) -> Edit) -> Element<'a, Message> {
    described_setting(name, hint, Toggle::new(on).on_toggle(Message::Edit(edit(!on))))
}

/// The line that says the switches above are saved, but nothing reads them yet.
fn not_enforced<'a>() -> Element<'a, Message> {
    text(lang::t("settings.privacy.not_enforced"))
        .size(12)
        .font(fonts::Inter::MEDIUM)
        .color(color::text_muted())
        .into()
}

/// The data actions: open the data folder, export the settings, clear the caches, and clear the local data in red.
fn data_actions<'a>() -> Element<'a, Message> {
    row![
        TextButton::labeled_icon(Icon::lucide().folder(), lang::t("settings.privacy.open_data"), Variant::Ghost).on_press(Message::OpenDataFolder),
        TextButton::labeled_icon(Icon::lucide().upload(), lang::t("settings.privacy.export"), Variant::Ghost).on_press(Message::ExportSettings),
        TextButton::labeled_icon(Icon::lucide().eraser(), lang::t("settings.privacy.clear_cache"), Variant::Ghost).on_press(Message::ClearCache),
        TextButton::labeled_icon(Icon::lucide().trash(), lang::t("settings.privacy.clear"), Variant::Danger).on_press(Message::ClearLocalData),
    ]
    .spacing(12)
    .into()
}
