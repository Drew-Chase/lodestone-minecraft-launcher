//! The timing name of each message: `update.<Variant>`, nested for the Library page, where the scroll and
//! hover messages are the ones that matter. Compiled in every build, since it is only a `match`.

use crate::app::LodestoneAppMessage;
use crate::app::pages::library::Message as LibraryMessage;

/// The scope name for `message`'s update, such as `update.Navigate` or `update.Library.Scrolled`.
///
/// The match is exhaustive, so a new message variant cannot go unnamed. It deliberately does not use
/// `Debug`: formatting `AppearanceReady(Ok(appearance))` would print the whole skin.
pub fn message_kind(message: &LodestoneAppMessage) -> &'static str {
    use LodestoneAppMessage as M;
    match message {
        M::FocusRequested => "update.FocusRequested",
        M::WindowCreation(_) => "update.WindowCreation",
        M::WindowStartResize(_) => "update.WindowStartResize",
        M::WindowSizeChanged(..) => "update.WindowSizeChanged",
        M::TitlebarMessage(_) => "update.TitlebarMessage",
        M::WindowMaximized(_) => "update.WindowMaximized",
        M::Navigate(_) => "update.Navigate",
        M::NavBack => "update.NavBack",
        M::NavForward => "update.NavForward",
        M::Login(_) => "update.Login",
        M::Library(message) => library_kind(message),
        M::Notifications(_) => "update.Notifications",
        M::News(_) => "update.News",
        M::Servers(_) => "update.Servers",
        M::ServersDialog(_) => "update.ServersDialog",
        M::JoinDialog(_) => "update.JoinDialog",
        M::CoopSync(_) => "update.CoopSync",
        M::Instance(_) => "update.Instance",
        M::PackUpdate(_) => "update.PackUpdate",
        M::Settings(_) => "update.Settings",
        M::Downloads(_) => "update.Downloads",
        M::Export(_) => "update.Export",
        M::ServerPack(_) => "update.ServerPack",
        M::Import(_) => "update.Import",
        M::Portable(_) => "update.Portable",
        M::Updater(_) => "update.Updater",
        M::DatabaseReady(_) => "update.DatabaseReady",
        M::Worlds(_) => "update.Worlds",
        M::World(_) => "update.World",
        M::WorldPinsLoaded(_) => "update.WorldPinsLoaded",
        M::InstancesLoaded(_) => "update.InstancesLoaded",
        M::InstanceDeleteAnswered { .. } => "update.InstanceDeleteAnswered",
        M::InstanceDeleted { .. } => "update.InstanceDeleted",
        M::InstanceSaved { .. } => "update.InstanceSaved",
        M::InstanceResetAnswered { .. } => "update.InstanceResetAnswered",
        M::SettingsSaved(_) => "update.SettingsSaved",
        M::DataFolderPicked(_) => "update.DataFolderPicked",
        M::DataFolderAnswered { .. } => "update.DataFolderAnswered",
        M::DataFolderMoved { .. } => "update.DataFolderMoved",
        M::VersionsPrefetched(_) => "update.VersionsPrefetched",
        M::SpeedTick => "update.SpeedTick",
        M::AccountSwitched(..) => "update.AccountSwitched",
        M::AccountMenu(_) => "update.AccountMenu",
        M::ProfileChangeConfirmed(_) => "update.ProfileChangeConfirmed",
        M::LocalDataAnswered(_) => "update.LocalDataAnswered",
        M::CacheAnswered(_) => "update.CacheAnswered",
        M::CacheCleared(_) => "update.CacheCleared",
        M::ProfileChanged(_) => "update.ProfileChanged",
        M::Skins(_) => "update.Skins",
        M::SkinsLoaded(_) => "update.SkinsLoaded",
        M::CapesLoaded(_) => "update.CapesLoaded",
        M::PreviewsReady(..) => "update.PreviewsReady",
        M::ProfileRefreshed(_) => "update.ProfileRefreshed",
        M::SkinFileMoved(_) => "update.SkinFileMoved",
        M::ReinstallFinished { .. } => "update.ReinstallFinished",
        M::FolderOpened(_) => "update.FolderOpened",
        M::SourceOpened(_) => "update.SourceOpened",
        M::ArtReady { .. } => "update.ArtReady",
        M::ResizeProbe => "update.ResizeProbe",
        M::AuthProbeTick => "update.AuthProbeTick",
        M::AuthProbeResult(_) => "update.AuthProbeResult",
        M::SidebarNav { .. } => "update.SidebarNav",
        M::AppearanceReady(_) => "update.AppearanceReady",
        M::CachedAppearance(_) => "update.CachedAppearance",
        M::CredentialsSaved(_) => "update.CredentialsSaved",
        M::Alert(_) => "update.Alert",
        M::Game(_) => "update.Game",
        M::SessionRefreshed(_) => "update.SessionRefreshed",
        M::Perf(_) => "update.Perf",
        M::Discover(_) => "update.Discover",
        M::SharedLoaded(_) => "update.SharedLoaded",
        M::ModSourcesLoaded(_) => "update.ModSourcesLoaded",
        M::AddedContentLoaded(_) => "update.AddedContentLoaded",
        M::PackActions(_) => "update.PackActions",
        M::Datapacks(_) => "update.Datapacks",
        M::ModActions(_) => "update.ModActions",
        M::UnlockAnswered { .. } => "update.UnlockAnswered",
        M::Resource(_) => "update.Resource",
        M::CloseRequested(_) => "update.CloseRequested",
        M::CloseAnswered(_) => "update.CloseAnswered",
        M::SweepCaches => "update.SweepCaches",
        M::ScaleFactor(_) => "update.ScaleFactor",
    }
}

/// The Library page's messages, each named under `update.Library.`.
fn library_kind(message: &LibraryMessage) -> &'static str {
    use LibraryMessage as L;
    match message {
        L::SearchChanged(_) => "update.Library.SearchChanged",
        L::FilterChanged(_) => "update.Library.FilterChanged",
        L::ViewChanged(_) => "update.Library.ViewChanged",
        L::Scrolled(_) => "update.Library.Scrolled",
        L::Play(_) => "update.Library.Play",
        L::LaunchNew(_) => "update.Library.LaunchNew",
        L::OpenInstance(_) => "update.Library.OpenInstance",
        L::EditInstance(_) => "update.Library.EditInstance",
        L::RevealInstance(_) => "update.Library.RevealInstance",
        L::Unlock(_) => "update.Library.Unlock",
        L::InstanceMenu { .. } => "update.Library.InstanceMenu",
        L::Hover { .. } => "update.Library.Hover",
        L::OpenJoinServer => "update.Library.OpenJoinServer",
        L::OpenCoopSync => "update.Library.OpenCoopSync",
        L::Notifications(_) => "update.Library.Notifications",
        L::UpdatePressed => "update.Library.UpdatePressed",
        L::Reload => "update.Library.Reload",
        L::OpenNewInstance => "update.Library.OpenNewInstance",
        L::OpenImport => "update.Library.OpenImport",
        L::NewInstance(_) => "update.Library.NewInstance",
    }
}

#[cfg(test)]
#[path = "../../../tests/app/perf/kind.rs"]
mod tests;
