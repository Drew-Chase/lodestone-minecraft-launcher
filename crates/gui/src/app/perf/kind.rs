//! The timing name of each message: `update.<Variant>`, nested for the Library page, where the scroll and
//! hover messages are the ones that matter. Compiled in every build, since it is only a `match`.

use crate::app::LodestoneAppMessage;
use crate::app::pages::library::Message as LibraryMessage;

/// The scope name for `message`'s update, such as `update.Navigate` or `update.Library.Scrolled`.
///
/// The match is exhaustive, so a new message variant cannot go unnamed. It deliberately does not use
/// `Debug`: formatting `FaceReady(Ok(bytes))` would print the whole skin.
pub fn message_kind(message: &LodestoneAppMessage) -> &'static str {
    use LodestoneAppMessage as M;
    match message {
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
        M::Instance(_) => "update.Instance",
        M::Settings(_) => "update.Settings",
        M::Showcase(_) => "update.Showcase",
        M::DatabaseReady(_) => "update.DatabaseReady",
        M::InstancesLoaded(_) => "update.InstancesLoaded",
        M::InstanceDeleteAnswered { .. } => "update.InstanceDeleteAnswered",
        M::InstanceDeleted { .. } => "update.InstanceDeleted",
        M::FolderOpened(_) => "update.FolderOpened",
        M::ResizeProbe => "update.ResizeProbe",
        M::RestoreStart { .. } => "update.RestoreStart",
        M::AuthProbeTick => "update.AuthProbeTick",
        M::AuthProbeResult(_) => "update.AuthProbeResult",
        M::SidebarNav { .. } => "update.SidebarNav",
        M::FaceReady(_) => "update.FaceReady",
        M::CredentialsSaved(_) => "update.CredentialsSaved",
        M::Alert(_) => "update.Alert",
        M::Game(_) => "update.Game",
        M::SessionRefreshed(_) => "update.SessionRefreshed",
        M::Perf(_) => "update.Perf",
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
        L::InstanceMenu { .. } => "update.Library.InstanceMenu",
        L::Hover { .. } => "update.Library.Hover",
        L::QuickAction(_) => "update.Library.QuickAction",
        L::Reload => "update.Library.Reload",
        L::OpenNewInstance => "update.Library.OpenNewInstance",
        L::NewInstance(_) => "update.Library.NewInstance",
    }
}

#[cfg(test)]
#[path = "../../../tests/app/perf/kind.rs"]
mod tests;
