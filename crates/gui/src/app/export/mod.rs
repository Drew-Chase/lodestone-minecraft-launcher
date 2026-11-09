//! The export dialog and the root's side of it. The Export action opens the dialog for an instance. Pressing Export
//! asks where to save the package, then queues the export in the downloads, so the popover shows it the way it shows
//! an install, and the work runs off the UI thread. The dialog is held by the root, beside the alerts, so the library's
//! menus and the instance page share it.

mod dialog;
pub mod job;
mod tree;

pub use dialog::Dialog;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use iced::Task;
use iced::widget::text_editor;
use lodestone_export::format::Format;

use super::{LodestoneAppMessage, LodestoneLauncher};
use crate::app::components::alert::Alert;
use crate::app::downloads;
use crate::app::lang;
use job::ExportJob;

/// The formats the dialog offers, in the order its picker lists them.
pub const FORMATS: [Format; 5] = [Format::Mrpack, Format::CurseForge, Format::Zip, Format::Lodestone, Format::Portable];

/// The dialog's messages: its fields, its folder listing, and the export itself.
#[derive(Debug, Clone)]
pub enum Message {
    /// Closes the dialog without exporting.
    Close,
    NameChanged(String),
    VersionChanged(String),
    DescriptionEdited(text_editor::Action),
    /// The format picker chose the format at this index of [`FORMATS`].
    FormatSelected(usize),
    /// The mods choice picked this index: [`dialog::MODS_ONLINE`] or [`dialog::MODS_OFFLINE`].
    ModsSelected(usize),
    /// A row's checkbox was pressed.
    Toggle(String),
    /// The header's checkbox was pressed.
    ToggleAll,
    /// A folder was opened, from its name, its chevron, or the breadcrumb.
    Open(String),
    /// A column heading was pressed.
    Sort(tree::Column),
    /// A folder's listing was read, or could not be.
    Listed {
        folder: String,
        result: Result<Vec<tree::Row>, String>,
    },
    /// Export was pressed: the save dialog opens.
    Submit,
    /// The save dialog answered with a path, or `None` when it was cancelled.
    PathChosen(Option<PathBuf>),
}

/// The label of a format, as the picker and the popover show it.
pub fn format_label(format: Format) -> &'static str {
    match format {
        Format::Mrpack => lang::t("export.format.mrpack"),
        Format::CurseForge => lang::t("export.format.curseforge"),
        Format::Zip => lang::t("export.format.zip"),
        Format::Lodestone => lang::t("export.format.lodestone"),
        Format::Portable => lang::t("export.format.portable"),
    }
}

/// The extension a package's file name carries. A portable launcher is a program on Windows, a zip of an app bundle
/// on macOS, and a bare file on Linux.
pub fn extension(format: Format) -> &'static str {
    match format {
        Format::Mrpack => "mrpack",
        Format::CurseForge | Format::Zip => "zip",
        Format::Lodestone => "lodestone",
        Format::Portable if cfg!(windows) => "exe",
        Format::Portable if cfg!(target_os = "macos") => "zip",
        Format::Portable => "",
    }
}

impl LodestoneLauncher {
    /// Opens the export dialog for the instance `id` and reads its top-level folder. An instance that is running is
    /// refused, since its files are changing under the export.
    pub(super) fn open_export(&mut self, id: &str) -> Task<LodestoneAppMessage> {
        let Some(instance) = self.library.instance(id) else {
            return Task::none();
        };
        // An export reads the instance's folder, which an in-place update is changing.
        if self.downloads.update_for_instance(id).is_some() {
            return self.alerts.show(Alert::warn(lang::t("pack_update.busy.working"))).discard();
        }
        if self.games.is_active(id) {
            let message = lang::t_with("export.running", &[("name", instance.name.as_str())]);
            return self.alerts.show(Alert::warn(message)).discard();
        }
        let folder = self.paths.instance(&instance.directory_name);
        let mut dialog = Dialog::new(&instance, folder);
        let listing = dialog.listing();
        self.export = Some(dialog);
        listing.map(LodestoneAppMessage::Export)
    }

    /// Applies a dialog message. Export and close belong to the root, since they touch the queue and the alerts.
    pub(super) fn export_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::Close => {
                self.export = None;
                Task::none()
            }
            Message::Submit => self.submit_export(),
            Message::PathChosen(Some(target)) => self.start_export(target),
            Message::PathChosen(None) => Task::none(),
            message => match self.export.as_mut() {
                Some(dialog) => dialog.update(message).map(LodestoneAppMessage::Export),
                None => Task::none(),
            },
        }
    }

    /// Asks where the package is saved, for a dialog that is ready to export.
    fn submit_export(&self) -> Task<LodestoneAppMessage> {
        let Some(dialog) = self.export.as_ref().filter(|dialog| dialog.ready()) else {
            return Task::none();
        };
        let (file_name, format) = (dialog.file_name(), dialog.format);
        Task::perform(choose_target(file_name, format), |target| {
            LodestoneAppMessage::Export(Message::PathChosen(target))
        })
    }

    /// Queues the export the dialog describes to `target`, closes the dialog, and opens the popover so the export shows.
    /// A target inside the instance is refused, since the export would read its own output.
    fn start_export(&mut self, target: PathBuf) -> Task<LodestoneAppMessage> {
        let Some(dialog) = self.export.as_ref() else {
            return Task::none();
        };
        if target.starts_with(&dialog.folder) {
            return self.alerts.show(Alert::error(lang::t("export.inside_instance"))).discard();
        }
        let request = match dialog.request(target) {
            Ok(request) => request,
            Err(error) => {
                return self
                    .alerts
                    .show(Alert::error(lang::t_with("export.failed", &[("error", error.as_str())])))
                    .discard();
            }
        };
        let Some(db) = self.db.clone() else {
            return self.alerts.show(Alert::error(lang::t("export.database_missing"))).discard();
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let export = ExportJob {
            name: dialog.instance_name.clone(),
            format: request.format,
            target: request.target.clone(),
            cancel: Arc::clone(&cancel),
            progress: None,
        };
        self.export = None;
        let id = self.downloads.enqueue_export(export);
        self.downloads.set_open(true);
        Task::sip(
            job::run(request, db, cancel),
            move |progress| LodestoneAppMessage::Downloads(downloads::Message::ExportProgress { job: id, progress }),
            move |result| LodestoneAppMessage::Downloads(downloads::Message::ExportFinished { job: id, result }),
        )
    }
}

/// The place the package is saved to, picked by the user. `None` when they cancel.
async fn choose_target(file_name: String, format: Format) -> Option<PathBuf> {
    let extension = extension(format);
    let mut picker = rfd::AsyncFileDialog::new()
        .set_title(lang::t("export.save_title"))
        .set_file_name(file_name);
    if !extension.is_empty() {
        picker = picker.add_filter(format_label(format), &[extension]);
    }
    picker.save_file().await.map(|file| file.path().to_path_buf())
}

#[cfg(test)]
#[path = "../../../tests/app/export/mod.rs"]
mod tests;
