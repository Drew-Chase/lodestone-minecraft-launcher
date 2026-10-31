//! The Screenshots tab's actions the root carries out: copying a capture's image, opening its file,
//! showing its folder, and moving it to the trash once the user has said so. The grid and the viewer
//! ask for them. The alerts, the clipboard and the file manager belong to the root.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use arboard::ImageData;
use iced::Task;
use tracing::{info, warn};

use super::instances::reveal_folder;
use super::{LodestoneAppMessage, LodestoneLauncher, clipboard};
use crate::app::components::alert::Alert;
use crate::app::lang;
use crate::app::pages::instance::Message as InstanceMessage;
use crate::app::pages::instance::files;
use crate::app::pages::instance::screenshots::{self, Action};

impl LodestoneLauncher {
    /// Carries out the menu row or viewer button the Screenshots tab picked. A separator carries no action.
    pub(super) fn screenshot_action(&mut self, path: PathBuf, action: Option<Action>) -> Task<LodestoneAppMessage> {
        match action {
            Some(Action::Copy) => self.copy_screenshot(path),
            Some(Action::Open) => open_file(path),
            Some(Action::Reveal) => match self.instance_page.instance() {
                Some(instance) => reveal_folder(self.paths.instance(&instance.directory_name).join("screenshots")),
                None => Task::none(),
            },
            Some(Action::Delete) => self.ask_screenshot_delete(path),
            None => Task::none(),
        }
    }

    /// Copies the capture's original pixels to the clipboard. The decode runs on the blocking pool, and
    /// the clipboard thread takes the image. The outcome returns as `Copied`.
    fn copy_screenshot(&mut self, path: PathBuf) -> Task<LodestoneAppMessage> {
        let sender = self.clipboard.sender();
        Task::perform(
            async move {
                let pixels = tokio::task::spawn_blocking(move || files::decode_rgba(&path))
                    .await
                    .map_err(|error| error.to_string())??;
                let (width, height) = pixels.dimensions();
                let image = ImageData {
                    width: width as usize,
                    height: height as usize,
                    bytes: Cow::Owned(pixels.into_raw()),
                };
                clipboard::copy(sender, image).await
            },
            |result| LodestoneAppMessage::Instance(InstanceMessage::Screenshots(screenshots::Message::Copied(result))),
        )
    }

    /// Reports a copy that failed. A copy that worked needs no word.
    pub(super) fn screenshot_copied(&mut self, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(()) => Task::none(),
            Err(error) => {
                warn!("Copying a screenshot failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with(
                        "instance.screenshots.copy.failed",
                        &[("error", error.as_str())],
                    )))
                    .discard()
            }
        }
    }

    /// Reports a capture that could not be opened in its own app.
    pub(super) fn screenshot_opened(&mut self, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(()) => Task::none(),
            Err(error) => {
                warn!("Opening a screenshot failed: {error}");
                self.alerts
                    .show(Alert::error(lang::t_with(
                        "instance.screenshots.open.failed",
                        &[("error", error.as_str())],
                    )))
                    .discard()
            }
        }
    }

    /// Asks before the capture goes to the trash. The answer returns as `DeleteAnswered`.
    fn ask_screenshot_delete(&mut self, path: PathBuf) -> Task<LodestoneAppMessage> {
        let name = file_name(&path);
        let message = lang::t_with("instance.screenshots.delete.message", &[("name", name.as_str())]);
        let answered = path.clone();
        self.alerts
            .show(Alert::danger(message).title(lang::t("instance.screenshots.delete.title")))
            .map(move |confirmed| {
                LodestoneAppMessage::Instance(InstanceMessage::Screenshots(screenshots::Message::DeleteAnswered {
                    path: answered.clone(),
                    confirmed,
                }))
            })
    }

    /// Moves the capture to the trash once the user has said yes. The result returns as `Deleted`.
    pub(super) fn answer_screenshot_delete(&mut self, path: PathBuf, confirmed: bool) -> Task<LodestoneAppMessage> {
        if !confirmed {
            return Task::none();
        }
        let deleted = path.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || files::move_to_trash(&path))
                    .await
                    .map_err(|error| error.to_string())?
            },
            move |result| {
                LodestoneAppMessage::Instance(InstanceMessage::Screenshots(screenshots::Message::Deleted {
                    path: deleted.clone(),
                    result,
                }))
            },
        )
    }

    /// Drops a capture that went to the trash from the tab. A failure is reported.
    pub(super) fn screenshot_deleted(&mut self, path: PathBuf, result: Result<(), String>) -> Task<LodestoneAppMessage> {
        match result {
            Ok(()) => {
                info!("Moved screenshot {} to the trash", path.display());
                self.instance_page.remove_screenshot(&path).map(LodestoneAppMessage::Instance)
            }
            Err(error) => {
                warn!("Moving screenshot {} to the trash failed: {error}", path.display());
                let name = file_name(&path);
                self.alerts
                    .show(Alert::error(lang::t_with(
                        "instance.screenshots.delete.failed",
                        &[("name", name.as_str()), ("error", error.as_str())],
                    )))
                    .discard()
            }
        }
    }
}

/// Opens the capture in the app the system gives PNG files, on the blocking pool. The outcome returns as
/// `Opened`.
fn open_file(path: PathBuf) -> Task<LodestoneAppMessage> {
    Task::perform(
        async move {
            match tokio::task::spawn_blocking(move || open::that_detached(&path)).await {
                Ok(opened) => opened.map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            }
        },
        |result| LodestoneAppMessage::Instance(InstanceMessage::Screenshots(screenshots::Message::Opened(result))),
    )
}

/// The name of the file at `path`, for the alerts.
pub(super) fn file_name(path: &Path) -> String {
    path.file_name()
        .map_or_else(|| path.display().to_string(), |name| name.to_string_lossy().into_owned())
}
