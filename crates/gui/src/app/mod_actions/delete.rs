//! Deleting a mod: asking first, moving its jar to the trash, and forgetting what was known of it.

use super::*;

impl LodestoneLauncher {
    /// Asks before a mod's jar goes to the trash. The answer returns as `DeleteAnswered`. A game running on the instance
    /// may have the jar open, so the delete waits until it stops.
    pub(super) fn ask_mod_delete(&mut self, file_name: String) -> Task<LodestoneAppMessage> {
        if self.shown_locked() {
            return Task::none();
        }
        let Some(context) = self.mods_context() else {
            return Task::none();
        };
        if self.games.is_active(&context.instance.id) {
            return self.alerts.show(Alert::warn(lang::t("instance.mods.running"))).discard();
        }
        let name = self
            .instance_page
            .mods()
            .row(&file_name)
            .map_or_else(|| file_name.clone(), |row| row.file.name.clone());
        let message = lang::t_with("instance.mods.delete.message", &[("name", name.as_str())]);
        let generation = self.instance_page.generation();
        let answered = file_name.clone();
        self.alerts
            .show(Alert::danger(message).title(lang::t("instance.mods.delete.title")))
            .map(move |confirmed| {
                LodestoneAppMessage::ModActions(Message::DeleteAnswered {
                    generation,
                    file_name: answered.clone(),
                    confirmed,
                })
            })
    }

    /// Moves the jar to the trash once the user has said yes. The result returns as `Deleted`.
    pub(super) fn answer_mod_delete(&mut self, file_name: String, confirmed: bool) -> Task<LodestoneAppMessage> {
        let Some(context) = self.mods_context() else {
            return Task::none();
        };
        if !confirmed {
            return Task::none();
        }
        let generation = self.instance_page.generation();
        self.instance_page.mods_mut().set_busy(&file_name, true);
        let path = context.mods_dir().join(&file_name);
        let answered = file_name.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || files::move_to_trash(&path))
                    .await
                    .map_err(|error| error.to_string())?
            },
            move |result| {
                LodestoneAppMessage::ModActions(Message::Deleted {
                    generation,
                    file_name: answered.clone(),
                    result,
                })
            },
        )
    }

    /// A jar was moved to the trash. Its record goes with it, and the folder is read again.
    pub(super) fn mod_deleted(&mut self, file_name: String, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        self.instance_page.mods_mut().set_busy(&file_name, false);
        match result {
            Ok(()) => {
                let Some(context) = self.mods_context() else {
                    return Task::none();
                };
                self.instance_page.mods_mut().forget_source(&file_name);
                self.discover.forget_source(&context.instance.id, &file_name);
                self.forget_checked(&context.instance.id, &file_name);
                Task::batch([self.forget_record(&context, file_name), self.rescan_mods(), self.load_instances()])
            }
            Err(error) => {
                warn!("moving a mod to the trash failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with("instance.mods.delete.failed", &[("error", error.as_str())])))
                    .discard()
            }
        }
    }

    /// Forgets the update the session remembers for a jar that is gone or was replaced.
    pub(super) fn forget_checked(&mut self, instance: &str, file_name: &str) {
        if let Some(check) = self.mod_checks.get_mut(instance) {
            check.updates.remove(file_name);
        }
    }

    /// Forgets the record of a jar that is gone.
    pub(super) fn forget_record(&self, context: &Context, file_name: String) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let instance = context.instance.id.clone();
        Task::future(async move {
            if let Err(error) = db.forget_mod_source(&instance, &file_name).await {
                warn!("forgetting a mod's source failed: {error}");
            }
        })
        .discard()
    }

    /// Reads the mods folder again after a change to it.
    pub(super) fn rescan_mods(&self) -> Task<LodestoneAppMessage> {
        let (Some(context), generation) = (self.mods_context(), self.instance_page.generation()) else {
            return Task::none();
        };
        let folder = context.folder;
        Task::perform(
            async move { tokio::task::spawn_blocking(move || files::scan_mods(&folder)).await.unwrap_or_default() },
            move |mods| LodestoneAppMessage::ModActions(Message::Rescanned { generation, mods }),
        )
    }
}
