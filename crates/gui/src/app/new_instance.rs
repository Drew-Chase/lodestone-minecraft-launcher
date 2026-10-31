//! The New Instance dialog's root side. Opening it loads its lists, and submitting it queues the install
//! with the downloads, which start, pause and finish it.

use iced::Task;

use super::{LodestoneAppMessage, LodestoneLauncher};
use crate::app::pages::library::Message as LibraryMessage;
use crate::app::pages::library::new_instance::{Message as DialogMessage, NewInstance};

impl LodestoneLauncher {
    /// Opens the dialog and loads the versions, runtimes and icon recents it offers.
    pub(super) fn open_new_instance(&mut self) -> Task<LodestoneAppMessage> {
        let mut tasks = vec![self.library.open_new_instance(self.paths.clone())];
        if let Some(db) = self.db.clone() {
            tasks.push(NewInstance::recents(db).map(LibraryMessage::NewInstance));
        }
        Task::batch(tasks).map(LodestoneAppMessage::Library)
    }

    /// Routes a dialog message. Submitting queues the install; the rest go to the dialog.
    pub(super) fn new_instance_message(&mut self, message: DialogMessage) -> Task<LodestoneAppMessage> {
        match message {
            DialogMessage::Submit => self.submit_new_instance(),
            message => self.library.new_instance_update(message).map(LodestoneAppMessage::Library),
        }
    }
}
