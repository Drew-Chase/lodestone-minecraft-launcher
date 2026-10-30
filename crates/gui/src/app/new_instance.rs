//! The New Instance dialog's root side. Opening it loads its lists, and starting,
//! finishing and cancelling an install need the database and the alerts, so those
//! messages are handled here.

use iced::Task;

use super::{LodestoneAppMessage, LodestoneLauncher};
use crate::app::components::alert::Alert;
use crate::app::pages::library::Message as LibraryMessage;
use crate::app::pages::library::instance::Instance;
use crate::app::pages::library::new_instance::{InstallEvent, Message as DialogMessage, NewInstance};

/// Wraps a dialog message for the root.
fn dialog(message: DialogMessage) -> LodestoneAppMessage {
    LodestoneAppMessage::Library(LibraryMessage::NewInstance(message))
}

impl LodestoneLauncher {
    /// Opens the dialog and loads the versions, runtimes and icon recents it offers.
    pub(super) fn open_new_instance(&mut self) -> Task<LodestoneAppMessage> {
        let mut tasks = vec![self.library.open_new_instance(self.paths.clone())];
        if let Some(db) = self.db.clone() {
            tasks.push(NewInstance::recents(db).map(LibraryMessage::NewInstance));
        }
        Task::batch(tasks).map(LodestoneAppMessage::Library)
    }

    /// Routes a dialog message. Starting, finishing and cancelling an install are handled
    /// here; the rest go to the dialog.
    pub(super) fn new_instance_message(&mut self, message: DialogMessage) -> Task<LodestoneAppMessage> {
        match message {
            DialogMessage::Submit => match self.db.clone() {
                Some(db) => self.library.start_new_instance(db).map(LodestoneAppMessage::Library),
                None => {
                    self.library.fail_new_instance("The launcher database is not ready yet.".to_string());
                    Task::none()
                }
            },
            DialogMessage::Install(InstallEvent::Finished(Ok(instance))) => self.finish_new_instance(*instance),
            DialogMessage::Install(InstallEvent::Finished(Err(error))) => {
                self.library.fail_new_instance(error.clone());
                self.alerts.show(Alert::error(format!("The instance was not created. {error}"))).discard()
            }
            DialogMessage::CancelInstall => self
                .alerts
                .show(Alert::warn("Cancel the install? The files it has written are removed.").yes_no())
                .map(|yes| dialog(DialogMessage::CancelAnswered(yes))),
            DialogMessage::CancelAnswered(true) => {
                self.library.cancel_new_instance();
                Task::none()
            }
            DialogMessage::CancelAnswered(false) => Task::none(),
            message => self.library.new_instance_update(message).map(LodestoneAppMessage::Library),
        }
    }

    /// Closes the dialog, reloads the library so the new instance shows, and says so.
    fn finish_new_instance(&mut self, instance: Instance) -> Task<LodestoneAppMessage> {
        self.library.close_new_instance();
        let announce = self.alerts.show(Alert::info(format!("{} is ready.", instance.name))).discard();
        Task::batch([self.load_instances(), announce])
    }
}
