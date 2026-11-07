//! How this copy of the launcher takes an update. It is worked out once, when the launcher starts, from where the launcher's
//! file is and whether that folder takes new files.

use std::path::PathBuf;

use lodestone_updater::{Payload, can_write_to, installed_with_installer};

use crate::app::paths;

/// How this copy takes an update.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// An installed Windows build. The release's setup runs silently and elevated, and the setup starts the launcher again.
    Installer,
    /// A Linux build in a folder that takes new files. The release's executable replaces this one.
    Executable(PathBuf),
    /// A macOS build in a folder that takes new app bundles. The release's bundle replaces this one.
    Bundle(PathBuf),
    /// A copy that cannot install an update itself: a Windows zip, a Linux build in a read-only folder, or a macOS app that
    /// runs from a read-only disk image. Its update button opens the release page.
    Manual,
}

impl Target {
    /// The target of the launcher that is running. A portable launcher never updates, so it has none.
    pub fn detect(portable: bool) -> Self {
        if portable {
            return Self::Manual;
        }
        let Ok(executable) = std::env::current_exe() else {
            return Self::Manual;
        };
        if cfg!(windows) {
            return if installed_with_installer(&executable) {
                Self::Installer
            } else {
                Self::Manual
            };
        }
        // A running file that has been replaced is reported as "<path> (deleted)", so only a file that is still there counts.
        if !executable.is_file() {
            return Self::Manual;
        }
        if let Some(bundle) = paths::app_bundle(&executable) {
            return match bundle.parent() {
                Some(folder) if can_write_to(folder) => Self::Bundle(bundle),
                _ => Self::Manual,
            };
        }
        match executable.parent() {
            Some(folder) if cfg!(target_os = "linux") && can_write_to(folder) => Self::Executable(executable),
            _ => Self::Manual,
        }
    }

    /// Whether this copy can install an update itself.
    pub fn installs(&self) -> bool {
        self.payload().is_some()
    }

    /// The file this copy's update comes from, or none for a copy that cannot install.
    pub fn payload(&self) -> Option<Payload> {
        match self {
            Self::Installer => Some(Payload::WindowsInstaller),
            Self::Executable(_) => Some(Payload::LinuxArchive),
            Self::Bundle(_) => Some(Payload::MacosArchive),
            Self::Manual => None,
        }
    }
}
