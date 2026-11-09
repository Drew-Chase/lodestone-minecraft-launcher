//! What the notifications are: the kinds the launcher raises, the drafts it records, and the rows the bell lists. A
//! notification's key names the event that raised it, so the database keeps one row per event.

use chrono::{DateTime, Local};

/// What a notification reports. The kind picks the row's icon, its text, and the button it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// An instance's game stopped with an error, and the player did not stop it.
    Crash,
    /// A newer version of an instance's modpack is out on the platform the pack came from.
    ModpackUpdate,
    /// Some mods of an instance have newer releases.
    ModUpdate,
    /// A newer release of the launcher is out.
    LauncherUpdate,
    /// An instance finished installing.
    InstallDone,
    /// An instance did not install.
    InstallFailed,
    /// An instance was moved onto a release of its modpack, in place.
    PackUpdated,
    /// An export wrote its package.
    ExportDone,
    /// An export did not write its package.
    ExportFailed,
}

impl Kind {
    /// Every kind, in the order the bell's documentation lists them.
    pub const ALL: [Kind; 9] = [
        Kind::Crash,
        Kind::ModpackUpdate,
        Kind::ModUpdate,
        Kind::LauncherUpdate,
        Kind::InstallDone,
        Kind::InstallFailed,
        Kind::PackUpdated,
        Kind::ExportDone,
        Kind::ExportFailed,
    ];

    /// The name the database keeps the kind under.
    pub fn to_db(self) -> &'static str {
        match self {
            Kind::Crash => "crash",
            Kind::ModpackUpdate => "modpack_update",
            Kind::ModUpdate => "mod_update",
            Kind::LauncherUpdate => "launcher_update",
            Kind::InstallDone => "install_done",
            Kind::InstallFailed => "install_failed",
            Kind::PackUpdated => "pack_updated",
            Kind::ExportDone => "export_done",
            Kind::ExportFailed => "export_failed",
        }
    }

    /// The kind a stored name stands for, or `None` for a name this build does not know.
    pub fn from_db(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.to_db() == name)
    }
}

/// A notification waiting to be recorded. Its `key` names the event, and the database keeps one row per key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    pub key: String,
    pub kind: Kind,
    /// What the notification is about: an instance's name, a release's tag, or a file's name.
    pub subject: String,
    /// The text the body needs: an exit code, a version, a count or an error. Empty when there is none.
    pub detail: String,
    /// The instance the notification is about, when it is about one.
    pub instance_id: Option<String>,
    /// Where the notification's button leads: a file, a folder, or a modpack's `platform/project`.
    pub target: Option<String>,
    pub created_at: DateTime<Local>,
}

/// A notification the database keeps, as the bell lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    pub id: i64,
    pub kind: Kind,
    pub subject: String,
    pub detail: String,
    pub instance_id: Option<String>,
    pub target: Option<String>,
    pub created_at: DateTime<Local>,
    pub read: bool,
}

/// The key of an event that can repeat under the same name, such as a crash of the same instance or an export of
/// the same job id after a restart. The moment it happened, to the millisecond, keeps each occurrence apart.
pub fn event_key(event: &str, at: DateTime<Local>) -> String {
    format!("{event}:{}", at.timestamp_millis())
}

#[cfg(test)]
#[path = "../../../tests/app/notifications/model.rs"]
mod tests;
