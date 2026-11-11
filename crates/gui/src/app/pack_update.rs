//! The root's side of the Versions tab: moving an instance onto a release of its modpack, and recording the release it is on. The tab lists
//! the releases and sends the actions. A move is read and reviewed, and the migration wizard shows the review; the update starts from the
//! wizard's decisions. Recording a release changes no file, so it queues at once. The update's stages are in `install::update`.

pub mod wizard;

use iced::Task;
use lodestone_platforms::{ContentType, ContentVersion};

use super::LodestoneAppMessage;
use super::LodestoneLauncher;
use super::components::alert::Alert;
use super::discover::pack_of_release;
use super::install::InstallProgress;
use super::install::pack::{Release, loader_of, payload_of};
use super::install::update::review::{Review, review as review_update};
use super::install::update::{Decisions, UpdateRequest};
use super::lang;
use super::pages::discover::Source;
use super::pages::library::instance::Instance;
use super::paths::LauncherPaths;
use super::shared_library;
use wizard::PackWizard;

/// The messages the Versions tab's actions and their updates leave for the root.
#[derive(Debug, Clone)]
pub enum Message {
    /// The release a move installs was read: its review starts, or the release could not be read.
    Read(Result<Box<UpdateRequest>, String>),
    /// The review reports its progress. The tab shows that the release is being prepared, so the reports are not drawn.
    Progress(InstallProgress),
    /// The review of a move is ready for the wizard, or it could not be made.
    Reviewed(Result<Box<Review>, String>),
    /// The release a recording reads is ready to queue, or it could not be read.
    Prepared(Result<Box<UpdateRequest>, String>),
    /// A message of the migration wizard.
    Wizard(wizard::Message),
}

impl LodestoneLauncher {
    /// Routes a message of this module.
    pub(super) fn pack_update_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::Read(result) => self.start_review(result),
            Message::Progress(_) => Task::none(),
            Message::Reviewed(result) => self.open_wizard(result),
            Message::Prepared(result) => self.queue_pack_update(result),
            Message::Wizard(message) => self.wizard_message(message),
        }
    }

    /// Moves the shown instance onto `release`: the release is read and reviewed, and the migration wizard opens on the review.
    pub(super) fn review_move(&mut self, release: String) -> Task<LodestoneAppMessage> {
        self.prepare_release(release, false)
    }

    /// Records `release` as the one the shown instance is on. It changes no file, so it queues without a review.
    pub(super) fn record_release(&mut self, release: String) -> Task<LodestoneAppMessage> {
        self.prepare_release(release, true)
    }

    /// Reads `release` into the update of the shown instance. A move's answer is `Read`, and a recording's is `Prepared`.
    fn prepare_release(&mut self, release: String, record_only: bool) -> Task<LodestoneAppMessage> {
        if let Some(reason) = self.versions_busy() {
            return self.alerts.show(Alert::warn(reason)).discard();
        }
        let Some(instance) = self.instance_page.instance().cloned() else {
            return Task::none();
        };
        let Some(source) = instance.source_platform.as_deref().and_then(Source::from_key) else {
            return Task::none();
        };
        let Some(version) = self.instance_page.versions().release(&release).cloned() else {
            return Task::none();
        };
        self.instance_page.versions_mut().set_preparing(Some(release));
        let paths = self.paths.clone();
        Task::perform(read_release(paths, instance, source, version, record_only), move |result| {
            let result = result.map(Box::new);
            LodestoneAppMessage::PackUpdate(if record_only { Message::Prepared(result) } else { Message::Read(result) })
        })
    }

    /// Reviews a move's release: it is staged and planned as the update would plan it. The review's answer opens the wizard.
    fn start_review(&mut self, result: Result<Box<UpdateRequest>, String>) -> Task<LodestoneAppMessage> {
        let request = match result {
            Ok(request) => request,
            Err(error) => {
                self.instance_page.versions_mut().set_preparing(None);
                return self
                    .alerts
                    .show(Alert::error(lang::t_with("pack_update.failed", &[("error", error.as_str())])))
                    .discard();
            }
        };
        let Some(db) = self.db.clone() else {
            self.instance_page.versions_mut().set_preparing(None);
            return Task::none();
        };
        Task::sip(
            review_update(*request, db),
            |progress| LodestoneAppMessage::PackUpdate(Message::Progress(progress)),
            |result| LodestoneAppMessage::PackUpdate(Message::Reviewed(result.map(Box::new))),
        )
    }

    /// Opens the migration wizard on a finished review, or reports why the review could not be made.
    fn open_wizard(&mut self, result: Result<Box<Review>, String>) -> Task<LodestoneAppMessage> {
        self.instance_page.versions_mut().set_preparing(None);
        match result {
            Ok(review) => {
                self.pack_wizard = Some(PackWizard::new(*review));
                Task::none()
            }
            Err(error) => self
                .alerts
                .show(Alert::error(lang::t_with("pack_update.failed", &[("error", error.as_str())])))
                .discard(),
        }
    }

    /// Handles a message of the migration wizard. Closing it discards the release the review staged. Applying it queues the update with
    /// the decisions the wizard holds, and every other message only changes the wizard.
    fn wizard_message(&mut self, message: wizard::Message) -> Task<LodestoneAppMessage> {
        match message {
            wizard::Message::Close => {
                let Some(wizard) = self.pack_wizard.take() else {
                    return Task::none();
                };
                let staged = self.paths.pack_journal(&wizard.review().request.instance.id);
                Task::future(async move {
                    let _ = tokio::task::spawn_blocking(move || std::fs::remove_dir_all(staged)).await;
                })
                .discard()
            }
            wizard::Message::Apply => {
                let Some(wizard) = self.pack_wizard.take() else {
                    return Task::none();
                };
                self.downloads.enqueue_update(wizard.into_request());
                self.downloads.set_open(true);
                self.sync_pending();
                self.start_ready_downloads()
            }
            message => {
                if let Some(wizard) = self.pack_wizard.as_mut() {
                    wizard.update(message);
                }
                Task::none()
            }
        }
    }

    /// Queues a prepared recording, or reports why its release could not be read.
    fn queue_pack_update(&mut self, result: Result<Box<UpdateRequest>, String>) -> Task<LodestoneAppMessage> {
        self.instance_page.versions_mut().set_preparing(None);
        match result {
            Ok(request) => {
                self.downloads.enqueue_update(*request);
                self.downloads.set_open(true);
                self.sync_pending();
                self.start_ready_downloads()
            }
            Err(error) => self
                .alerts
                .show(Alert::error(lang::t_with("pack_update.failed", &[("error", error.as_str())])))
                .discard(),
        }
    }

    /// Whether an in-place update is writing the shown instance, or waits to.
    pub(super) fn shown_instance_updating(&self) -> bool {
        self.instance_page
            .instance()
            .is_some_and(|instance| self.downloads.update_for_instance(&instance.id).is_some())
    }

    /// Why the shown instance cannot be updated now, when it cannot: its game is running, or a job for it is still going.
    pub(super) fn versions_busy(&self) -> Option<&'static str> {
        let instance = self.instance_page.instance()?;
        let id = instance.id.as_str();
        // A locked instance is on the release its pack set, so no release is moved to or recorded until it is unlocked.
        if instance.locked {
            Some(lang::t("pack_update.busy.locked"))
        } else if self.games.is_active(id) {
            Some(lang::t("pack_update.busy.playing"))
        } else if self.downloads.job_for_instance(id).is_some() {
            Some(lang::t("pack_update.busy.working"))
        } else {
            None
        }
    }
}

/// Reads the release an update installs: its modpack and archive, the payload the update applies, and the Minecraft version and
/// loader the release names. The instance is taken into the request, which the queue then runs.
async fn read_release(
    paths: LauncherPaths,
    instance: Instance,
    source: Source,
    version: ContentVersion,
    record_only: bool,
) -> Result<UpdateRequest, String> {
    let project = instance
        .source_project_id
        .clone()
        .ok_or_else(|| lang::t("pack_update.no_project").to_string())?;
    let folder = shared_library::download_folder(source.key(), &project, &version.id);
    let shared = shared_library::folder(&paths, ContentType::Modpack).join(folder);
    let (modpack, archive) = pack_of_release(source, &instance.name, &version, &shared).await?;
    let payload = payload_of(&modpack, archive, Some(source)).await?;
    let mc_version = if modpack.minecraft.is_empty() {
        version.minecraft_versions.first().cloned().unwrap_or_else(|| instance.mc_version.clone())
    } else {
        modpack.minecraft.clone()
    };
    let backup_worlds = !record_only && mc_version != instance.mc_version;
    Ok(UpdateRequest {
        paths,
        release: Release {
            id: version.id.clone(),
            number: version.version_number.clone(),
        },
        mc_version,
        loader: loader_of(modpack.loader),
        loader_version: modpack.loader_version.clone(),
        payload,
        backup_worlds,
        decisions: Decisions::default(),
        record_only,
        reviewed: None,
        instance,
    })
}
