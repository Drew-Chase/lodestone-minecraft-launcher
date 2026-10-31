//! The root's side of the queue: queueing the install the dialog describes, starting the queued installs
//! as slots free up, pausing, resuming and cancelling them, and the results they return. The database,
//! the alerts and the library belong to the root, so these sit beside the other root handlers in `app`.

use std::collections::HashSet;
use std::path::PathBuf;

use chrono::{Local, TimeDelta};
use iced::Task;
use tracing::{info, warn};

use super::Message;
use crate::app::components::alert::Alert;
use crate::app::database::Database;
use crate::app::install;
use crate::app::lang;
use crate::app::pages::Page;
use crate::app::pages::library::instance::Instance;
use crate::app::{LodestoneAppMessage, LodestoneLauncher};

/// Wraps a queue message for the root.
fn queue(message: Message) -> LodestoneAppMessage {
    LodestoneAppMessage::Downloads(message)
}

/// Today's date and the first day of the week the totals count, as local `YYYY-MM-DD` dates.
fn day_keys() -> (String, String) {
    let now = Local::now();
    (
        now.format("%Y-%m-%d").to_string(),
        (now - TimeDelta::days(6)).format("%Y-%m-%d").to_string(),
    )
}

impl LodestoneLauncher {
    /// Queues the install the New Instance dialog describes, closes the dialog, and opens the popover so
    /// the install shows. A form that cannot install stays open and says why.
    pub(crate) fn submit_new_instance(&mut self) -> Task<LodestoneAppMessage> {
        if self.db.is_none() {
            self.library.fail_new_instance(lang::t("new_instance.database_missing").to_owned());
            return Task::none();
        }
        let taken = self.downloads.reserved_directories();
        let request = match self.library.new_instance_request(&taken) {
            Ok(request) => request,
            Err(error) => {
                self.library.fail_new_instance(error);
                return Task::none();
            }
        };
        self.library.close_new_instance();
        self.downloads.enqueue(request);
        self.downloads.set_open(true);
        self.sync_pending();
        Task::batch([self.start_ready_downloads(), self.refresh_totals()])
    }

    /// Starts the queued installs while running slots are free.
    pub(crate) fn start_ready_downloads(&mut self) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let mut tasks = Vec::new();
        while let Some(id) = self.downloads.next_start() {
            let (Some(request), Some(keep)) = (self.downloads.request(id).cloned(), self.downloads.keep_flag(id)) else {
                break;
            };
            let (task, abort) = Task::sip(
                install::install(request, db.clone(), keep),
                move |progress| queue(Message::Progress { job: id, progress }),
                move |result| {
                    queue(Message::Finished {
                        job: id,
                        result: result.map(Box::new),
                    })
                },
            )
            .abortable();
            self.downloads.start(id, abort);
            tasks.push(task);
        }
        self.sync_pending();
        Task::batch(tasks)
    }

    /// Applies a queue message: the installs' reports and results, and the popover's controls.
    pub(crate) fn downloads_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::Progress { job, progress } => {
                self.downloads.record(job, progress);
                self.sync_pending();
                Task::none()
            }
            Message::Finished { job, result } => self.install_finished(job, result),
            Message::TotalsLoaded(Ok(totals)) => {
                self.downloads.set_totals(totals);
                Task::none()
            }
            Message::TotalsLoaded(Err(error)) => {
                warn!("could not read the download totals: {error}");
                Task::none()
            }
            Message::TogglePanel => {
                let open = !self.downloads.is_open();
                self.downloads.set_open(open);
                if open { self.refresh_totals() } else { Task::none() }
            }
            Message::DismissPanel => {
                self.downloads.set_open(false);
                Task::none()
            }
            Message::OpenSettings => self.navigate(Page::Settings),
            Message::Pause(job) => self.pause_download(job),
            Message::Resume(job) => {
                self.downloads.resume(job);
                self.sync_pending();
                self.start_ready_downloads()
            }
            Message::TogglePauseAll => {
                if self.downloads.has_pausable() {
                    for handle in self.downloads.pause_all() {
                        handle.abort();
                    }
                    let bytes = self.downloads.take_all_bytes();
                    self.sync_pending();
                    Task::batch([self.count_bytes(bytes), self.start_ready_downloads()])
                } else {
                    self.downloads.resume_all();
                    self.sync_pending();
                    self.start_ready_downloads()
                }
            }
            Message::Cancel(job) => self
                .alerts
                .show(Alert::warn(lang::t("new_instance.cancel_confirm")).yes_no())
                .map(move |confirmed| queue(Message::CancelAnswered { job, confirmed })),
            Message::CancelAnswered { job, confirmed: true } => self.cancel_download(job),
            Message::CancelAnswered { confirmed: false, .. } => Task::none(),
            Message::ClearCompleted => {
                self.downloads.clear_completed();
                Task::none()
            }
        }
    }

    /// Ends a finished install. Its instance joins the library at once, the library reloads from the
    /// database, and a failure shows the reason in an alert as well as in the popover.
    fn install_finished(&mut self, job: u64, result: Result<Box<Instance>, String>) -> Task<LodestoneAppMessage> {
        let bytes = self.downloads.take_bytes(job);
        let mut tasks = vec![self.count_bytes(bytes), self.start_ready_downloads()];
        match result {
            Ok(instance) => {
                info!("Installed instance {}", instance.name);
                self.downloads.finish(job, Ok(()));
                self.library.add_instance(*instance);
                tasks.push(self.load_instances());
            }
            Err(error) => {
                self.downloads.finish(job, Err(error.clone()));
                tasks.push(
                    self.alerts
                        .show(Alert::error(lang::t_with("new_instance.not_created", &[("error", error.as_str())])))
                        .discard(),
                );
            }
        }
        self.sync_pending();
        Task::batch(tasks)
    }

    /// Pauses a job. A running install is stopped with its folder kept, and the bytes it moved are counted.
    fn pause_download(&mut self, job: u64) -> Task<LodestoneAppMessage> {
        if let Some(handle) = self.downloads.pause(job) {
            handle.abort();
        }
        let bytes = self.downloads.take_bytes(job);
        self.sync_pending();
        Task::batch([self.count_bytes(bytes), self.start_ready_downloads()])
    }

    /// Cancels a job after the user agreed. A running install is stopped and removes its own folder; a
    /// paused one has no install left to do that, so its folder goes here.
    fn cancel_download(&mut self, job: u64) -> Task<LodestoneAppMessage> {
        let bytes = self.downloads.take_bytes(job);
        let folder = self.downloads.cancel(job);
        self.sync_pending();
        let removal = folder.map_or_else(Task::none, remove_partial);
        Task::batch([removal, self.count_bytes(bytes), self.start_ready_downloads()])
    }

    /// Reads the day's and the week's totals from the database again.
    pub(crate) fn refresh_totals(&self) -> Task<LodestoneAppMessage> {
        self.count_bytes(0)
    }

    /// Counts `bytes` into today's total, then reads the totals again. With no bytes, it only reads them.
    fn count_bytes(&self, bytes: u64) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        Task::perform(
            async move {
                let (today, week_start) = day_keys();
                if bytes > 0 {
                    db.add_download_bytes(&today, bytes).await?;
                }
                db.download_totals(&today, &week_start).await
            },
            |result| queue(Message::TotalsLoaded(result)),
        )
    }

    /// Hands the library the installs that have not ended, so their cards and pages show them.
    pub(crate) fn sync_pending(&mut self) {
        self.library.set_pending(self.downloads.pending());
    }
}

/// Removes the folders of installs that never finished, once at startup. The sweep reads the instance rows
/// first, so a folder whose instance exists is kept. It runs as soon as the database opens, before the user
/// can open the New Instance dialog, so no install of this session can have a folder yet.
pub(crate) fn sweep_partial_installs(db: Database, instances_dir: PathBuf) -> Task<LodestoneAppMessage> {
    Task::future(async move {
        let known: HashSet<String> = match db.instances().await {
            Ok(instances) => instances.into_iter().map(|instance| instance.directory_name).collect(),
            Err(error) => {
                warn!("could not read the instances, so the unfinished installs are not swept: {error}");
                return;
            }
        };
        let removed = install::sweep_partial(instances_dir, known).await;
        if removed > 0 {
            info!("Removed {removed} unfinished install folder(s)");
        }
    })
    .discard()
}

/// Removes the folder of a paused install that was cancelled. A folder already gone counts as removed.
fn remove_partial(folder: PathBuf) -> Task<LodestoneAppMessage> {
    Task::future(async move {
        let removal = tokio::task::spawn_blocking(move || std::fs::remove_dir_all(&folder)).await;
        if let Ok(Err(error)) = removal
            && error.kind() != std::io::ErrorKind::NotFound
        {
            warn!("could not remove a cancelled install's folder: {error}");
        }
    })
    .discard()
}
