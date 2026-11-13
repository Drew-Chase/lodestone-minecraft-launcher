//! Switching a mod on or off. The switch renames the jar: one switched off gains `.disabled`, and one switched on loses
//! it. The record of its platform source and its cached hash follow the new name, so the mod keeps what is known of it.

use std::path::PathBuf;

use tracing::{info, warn};

use super::*;

impl LodestoneLauncher {
    /// A mod's switch was flipped. The jar is renamed on the blocking pool, and the row follows once the rename lands. A
    /// game running on the instance may have the jar open, so the switch waits until it stops, as a delete does.
    pub(in crate::app) fn mod_toggle(&mut self, file_name: String, enabled: bool) -> Task<LodestoneAppMessage> {
        if self.shown_locked() || self.instance_page.mods().is_busy(&file_name) {
            return Task::none();
        }
        let Some(to) = files::switched_name(&file_name, enabled) else {
            return Task::none();
        };
        let (Some(context), Some(db)) = (self.mods_context(), self.db.clone()) else {
            return Task::none();
        };
        if self.games.is_active(&context.instance.id) {
            return self.alerts.show(Alert::warn(lang::t("instance.mods.running"))).discard();
        }
        self.instance_page.mods_mut().set_busy(&file_name, true);
        let generation = self.instance_page.generation();
        let instance = context.instance.id.clone();
        let mods = context.mods_dir();
        let answered = file_name.clone();
        Task::perform(rename_mod(db, instance, mods, file_name, to), move |result| {
            LodestoneAppMessage::ModActions(Message::Toggled {
                generation,
                file_name: answered.clone(),
                enabled,
                result,
            })
        })
    }

    /// Applies a switch's outcome. A renamed jar's row takes its new name and switch position, its cached update moves
    /// with it, and the recorded sources are read again so the rows match the database. A failed rename is reported, and
    /// the switch keeps its position.
    pub(super) fn mod_toggled(&mut self, file_name: String, enabled: bool, result: Result<String, String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(renamed) => {
                info!("Renamed the mod {file_name} to {renamed}");
                if let Some(instance) = self.instance_page.instance().map(|instance| instance.id.clone()) {
                    self.discover.renamed(&instance, &file_name, &renamed);
                }
                self.instance_page.mods_mut().renamed(&file_name, &renamed, enabled);
                let mut save = Task::none();
                if let Some(instance) = self.instance_page.instance().map(|instance| instance.id.clone())
                    && let Some(check) = self.mod_checks.get_mut(&instance)
                    && let Some(release) = check.updates.remove(&file_name)
                {
                    check.updates.insert(renamed, release);
                    save = self.save_mod_check(&instance);
                }
                Task::batch([save, self.reload_sources(self.instance_page.generation())])
            }
            Err(error) => {
                warn!("Switching the mod {file_name} failed: {error}");
                self.instance_page.mods_mut().set_busy(&file_name, false);
                self.alerts
                    .show(Alert::error(lang::t_with("instance.mods.toggle_failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// Reads the instance's recorded sources again, as the Mods tab does when it is opened.
    fn reload_sources(&self, generation: u64) -> Task<LodestoneAppMessage> {
        let (Some(context), Some(db)) = (self.mods_context(), self.db.clone()) else {
            return Task::none();
        };
        let instance = context.instance.id;
        Task::perform(async move { db.mod_sources_of(&instance).await }, move |result| {
            LodestoneAppMessage::ModActions(Message::Sources { generation, result })
        })
    }
}

/// Renames the jar `from` to `to` in `mods`, then moves its records to the new name. A jar already called `to` is left
/// alone. If the records cannot follow, the jar goes back to its old name. Returns the name the jar has now.
async fn rename_mod(db: Database, instance: String, mods: PathBuf, from: String, to: String) -> Result<String, String> {
    let (source, target) = (mods.join(&from), mods.join(&to));
    let taken = to.clone();
    tokio::task::spawn_blocking(move || {
        if target.exists() {
            return Err(lang::t_with("instance.mods.toggle_exists", &[("name", taken.as_str())]));
        }
        std::fs::rename(&source, &target).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())??;
    if let Err(error) = db.rename_mod(&instance, &from, &to).await {
        let (back_from, back_to) = (mods.join(&to), mods.join(&from));
        if let Err(undo) = tokio::task::spawn_blocking(move || std::fs::rename(back_from, back_to))
            .await
            .unwrap_or_else(|error| Err(std::io::Error::other(error)))
        {
            warn!("Undoing the rename of the mod {from} failed: {undo}");
        }
        return Err(error);
    }
    Ok(to)
}
