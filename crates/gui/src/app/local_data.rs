//! Clearing what the launcher keeps on this machine: the saved settings and the sign-ins. The instances and their worlds
//! stay where they are.

use iced::Task;
use tracing::{info, warn};

use crate::app::components::alert::Alert;
use crate::app::lang;
use crate::app::preferences::{Preferences, SettingsValues};
use crate::app::storage::StoredCredentials;

use super::{LodestoneAppMessage, LodestoneLauncher};

impl LodestoneLauncher {
    /// Asks before the saved settings and the sign-ins are cleared. The answer returns as `LocalDataAnswered`.
    pub(super) fn ask_clear_local_data(&mut self) -> Task<LodestoneAppMessage> {
        self.alerts
            .show(Alert::danger(lang::t("settings.privacy.clear_body")).title(lang::t("settings.privacy.clear_title")))
            .map(LodestoneAppMessage::LocalDataAnswered)
    }

    /// Clears the saved settings and the sign-ins on a yes. The settings go back to their defaults, on disk and in the page.
    /// The sign-ins are emptied through the credential writer, so a write already in flight cannot land after this one.
    /// The launcher then signs out and shows the setup wizard again, since its choice was in the saved preferences.
    pub(super) fn local_data_answered(&mut self, confirmed: bool) -> Task<LodestoneAppMessage> {
        if !confirmed {
            return Task::none();
        }
        info!("Clearing the saved settings and the sign-ins");
        let remove_preferences = Task::future(async {
            match tokio::task::spawn_blocking(clear_preferences_keeping_folders).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => warn!("Removing the saved settings failed: {error}"),
                Err(error) => warn!("Removing the saved settings failed: {error}"),
            }
            // The faces kept on disk belong to the sign-ins, so they go with them.
            match tokio::task::spawn_blocking(crate::app::appearance_cache::clear).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => warn!("Removing the kept appearances failed: {error}"),
                Err(error) => warn!("Removing the kept appearances failed: {error}"),
            }
        })
        .discard();

        let defaults = SettingsValues::default();
        defaults.apply();
        self.settings_values = defaults;
        lang::set_active(lang::DEFAULT);
        self.settings.replace_saved(&self.settings_values, &self.paths);
        self.refresh_instance_page();

        self.credentials = StoredCredentials::default();
        self.account = None;
        self.face = None;
        self.skins = crate::app::pages::skins::Skins::default();
        self.pending_change = None;
        self.waiting_change = None;
        self.restoring = None;
        self.adding_account = false;
        // The setup choice is in the preferences that were just cleared, so the wizard starts again from its first step.
        self.setup_finished = false;
        self.setup = crate::app::pages::setup::Setup::default();
        let save = self.save_credentials();
        let page = self.signed_out_page();
        Task::batch([remove_preferences, save, self.navigate(page)])
    }
}

/// Removes the saved settings, but keeps the folders the data and the other files are in: they stay where they are, so the
/// launcher must still find them.
fn clear_preferences_keeping_folders() -> Result<(), String> {
    // One locked write resets every choice but the folders, so no other preference change can land between the read and the write.
    Preferences::update(|preferences| {
        *preferences = Preferences {
            data_folder: preferences.data_folder.take(),
            instances_folder: preferences.instances_folder.take(),
            logs_folder: preferences.logs_folder.take(),
            cache_folder: preferences.cache_folder.take(),
            ..Preferences::default()
        };
    })
}
