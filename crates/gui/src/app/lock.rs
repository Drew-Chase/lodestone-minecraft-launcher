//! The instance lock. A locked instance keeps its mods, its loader, its Minecraft version and its modpack release as they are
//! until the user unlocks it. A modpack installed from Discover starts locked, and any instance can be locked or unlocked. This
//! module saves a lock or an unlock, and asks before an unlock. Each change the lock refuses checks
//! [`LodestoneLauncher::shown_locked`] at its own entry point, as the controls that start it are disabled while the instance is
//! locked.

use iced::Task;

use super::LodestoneAppMessage;
use super::LodestoneLauncher;
use super::components::alert::Alert;
use super::lang;

impl LodestoneLauncher {
    /// Whether the instance the page shows is locked to its modpack.
    pub(super) fn shown_locked(&self) -> bool {
        self.instance_page.instance().is_some_and(|instance| instance.locked)
    }

    /// Asks before an instance is unlocked, since its mods and versions can change once it is. The answer returns as
    /// `UnlockAnswered`. Nothing is asked for an instance that is not locked, or one whose install has not ended.
    pub(super) fn ask_unlock(&mut self, id: String) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.library.instance(&id) else {
            return Task::none();
        };
        if !instance.locked || self.downloads.job_for_instance(&id).is_some() {
            return Task::none();
        }
        let message = lang::t_with("instance.lock.unlock.message", &[("name", instance.name.as_str())]);
        self.alerts
            .show(
                Alert::warn(message)
                    .title(lang::t("instance.lock.unlock.title"))
                    .labelled(lang::t("instance.lock.unlock.confirm"), Some(lang::t("common.cancel"))),
            )
            .map(move |confirmed| LodestoneAppMessage::UnlockAnswered { id: id.clone(), confirmed })
    }

    /// Locks the instance `id` to its modpack, so its mods and versions stop changing until it is unlocked. Locking only
    /// restricts, so nothing is asked.
    pub(super) fn lock_instance(&mut self, id: String) -> Task<LodestoneAppMessage> {
        self.set_lock(id, true)
    }

    /// Unlocks the instance once the user has said so.
    pub(super) fn unlock_answered(&mut self, id: String, confirmed: bool) -> Task<LodestoneAppMessage> {
        if !confirmed {
            return Task::none();
        }
        self.set_lock(id, false)
    }

    /// Sets the lock of the instance `id` and saves it. It is saved the way any edit of an instance is: the library copy and
    /// the page take the change at once, and the database write follows. An install that has not ended has no lock to set
    /// yet, and an instance already in the state asked for is left as it is.
    fn set_lock(&mut self, id: String, locked: bool) -> Task<LodestoneAppMessage> {
        let Some(mut instance) = self.library.instance(&id) else {
            return Task::none();
        };
        if instance.locked == locked || self.downloads.job_for_instance(&id).is_some() {
            return Task::none();
        }
        instance.locked = locked;
        let save = self.save_instance(instance, None);
        // Discover reads the lock from its own copy of the instances, so it takes the change now.
        self.sync_discover_instances();
        save
    }
}
