//! The first-run gate and the setup wizard's messages. While no account plays, the launcher shows the setup wizard on first run,
//! and the sign-in page after it. The wizard's Next, Back, Skip and Finish are carried out here, and so are the requests the
//! wizard makes of the root: its Discover step searches and prepares installs the way the Discover page does. Back and Forward
//! also pass over the sign-in pages while an account plays, since a signed-in player has no use for them.

use iced::Task;
use lodestone_platforms::{ContentType, SearchQuery, SearchSort};
use tracing::{info, warn};

use crate::app::components::alert::Alert;
use crate::app::discover::{prepare_modpack, run_search};
use crate::app::install::InstallRequest;
use crate::app::pages::Page;
use crate::app::pages::discover::Project;
use crate::app::pages::setup::Step;
use crate::app::pages::setup::discover::{self as discover_step, LIMIT};
use crate::app::preferences::Preferences;

use super::{LodestoneAppMessage, LodestoneLauncher};
use crate::app::pages::setup::Message as SetupMessage;

/// The page shown while no account plays: the sign-in page on a portable launcher, and once the setup is finished or skipped,
/// and the setup wizard otherwise.
fn gate_page(portable: bool, setup_finished: bool) -> Page {
    if portable || setup_finished { Page::Login } else { Page::Setup }
}

impl LodestoneLauncher {
    /// The page shown while no account plays. See `gate_page`.
    pub(super) fn signed_out_page(&self) -> Page {
        gate_page(self.portable.is_some(), self.setup_finished)
    }

    /// Steps back or forward through the history. While an account plays, the sign-in pages are passed over, so Back never shows
    /// the sign-in gate to a player who is already signed in. Returns whether the page changed.
    pub(super) fn step_history(&mut self, forward: bool) -> bool {
        let signed_in = self.account.is_some();
        let accepts = move |page: Page| !(signed_in && matches!(page, Page::Login | Page::Setup));
        if forward {
            self.router.go_forward_where(accepts)
        } else {
            self.router.go_back_where(accepts)
        }
    }

    /// Applies a message of the setup wizard. Its steps change on screen; the requests it makes run here.
    pub(super) fn setup_message(&mut self, message: SetupMessage) -> Task<LodestoneAppMessage> {
        // The sign-in step moves on only once an account is stored, so Next does nothing before a sign-in.
        if matches!(message, SetupMessage::Next) && self.setup.step() == Step::SignIn && self.account.is_none() {
            return Task::none();
        }
        match message {
            SetupMessage::Skip => {
                info!("The setup wizard was skipped");
                self.setup_finish()
            }
            SetupMessage::Finish => {
                info!("The setup wizard was finished");
                self.setup_finish()
            }
            SetupMessage::Next | SetupMessage::Back => {
                self.setup.update(message);
                self.setup_entered()
            }
            // The sign-in step's messages go to the sign-in page, through the root so the language picker's choice is saved too.
            SetupMessage::Login(message) => self.update(LodestoneAppMessage::Login(message)),
            SetupMessage::Folder(kind, action) => self.folder_message(kind, action),
            SetupMessage::Import(message) => self.setup_import_message(message),
            SetupMessage::Discover(message) => self.setup_discover_message(message),
        }
    }

    /// Starts what the step the wizard has just moved to needs. The Import step looks for the launchers on this machine, and the
    /// Discover step lists the popular modpacks when it has none to show.
    fn setup_entered(&mut self) -> Task<LodestoneAppMessage> {
        match self.setup.step() {
            Step::Import => self.setup_import_scan(),
            Step::Discover => {
                let state = self.setup.discover();
                if state.hits.is_empty() && !state.searching {
                    self.setup_discover_search()
                } else {
                    Task::none()
                }
            }
            Step::SignIn | Step::Folders | Step::Done => Task::none(),
        }
    }

    /// Applies a message of the Discover step: the search text, the search, and the install of one of its hits.
    fn setup_discover_message(&mut self, message: discover_step::Message) -> Task<LodestoneAppMessage> {
        match message {
            discover_step::Message::Query(query) => {
                self.setup.discover_mut().query = query;
                Task::none()
            }
            discover_step::Message::Search => self.setup_discover_search(),
            discover_step::Message::Results(result) => {
                let state = self.setup.discover_mut();
                state.searching = false;
                match result {
                    Ok(page) => {
                        state.error = None;
                        state.hits = page.hits;
                    }
                    Err(error) => {
                        state.error = Some(error);
                        state.hits.clear();
                    }
                }
                Task::none()
            }
            discover_step::Message::Install(project_id) => self.setup_discover_install(project_id),
            discover_step::Message::Prepared(project_id, result) => self.setup_discover_prepared(project_id, result),
        }
    }

    /// Searches the modpacks the Discover page searches, for the text the step holds. An empty search lists the most popular ones.
    fn setup_discover_search(&mut self) -> Task<LodestoneAppMessage> {
        let source = self.discover.source();
        let state = self.setup.discover_mut();
        if state.searching {
            return Task::none();
        }
        state.searching = true;
        state.error = None;
        let text = state.query.trim().to_string();
        let sort = if text.is_empty() { SearchSort::Popular } else { SearchSort::Relevance };
        let query = SearchQuery {
            text,
            sort,
            limit: LIMIT,
            ..SearchQuery::new(ContentType::Modpack)
        };
        Task::perform(async move { run_search(source, query).await.0 }, |result| {
            LodestoneAppMessage::Setup(SetupMessage::Discover(discover_step::Message::Results(result)))
        })
    }

    /// Prepares the install of one of the hits, as the Discover page prepares its own: the modpack is read and its install answers
    /// with `Prepared`, which queues it.
    fn setup_discover_install(&mut self, project_id: String) -> Task<LodestoneAppMessage> {
        let source = self.discover.source();
        let state = self.setup.discover_mut();
        if state.preparing.contains(&project_id) || state.queued.contains(&project_id) {
            return Task::none();
        }
        let Some(hit) = state.hits.iter().find(|hit| hit.id == project_id).cloned() else {
            return Task::none();
        };
        state.preparing.insert(project_id.clone());
        let project = Project {
            source,
            content: ContentType::Modpack,
            project_id: hit.id.clone(),
            name: hit.name.clone(),
            web_url: hit.web_url.clone(),
            icon_url: hit.icon_url.clone(),
            game_versions: Vec::new(),
            version_id: None,
        };
        let paths = self.paths.clone();
        let taken = self.taken_directories();
        Task::perform(
            async move {
                prepare_modpack(project, paths, taken)
                    .await
                    .map(|(request, _)| Box::new(request.into_locked()))
            },
            move |result| LodestoneAppMessage::Setup(SetupMessage::Discover(discover_step::Message::Prepared(project_id, result))),
        )
    }

    /// The install of a hit was prepared, or could not be. A prepared install is queued with the downloads, as the Discover page
    /// queues its own, and the hit then says it is queued.
    fn setup_discover_prepared(&mut self, project_id: String, result: Result<Box<InstallRequest>, String>) -> Task<LodestoneAppMessage> {
        self.setup.discover_mut().preparing.remove(&project_id);
        match result {
            Ok(request) => {
                self.setup.discover_mut().queued.insert(project_id);
                self.queue_install(*request)
            }
            Err(error) => {
                warn!("Preparing the install of {project_id} failed: {error}");
                self.alerts.show(Alert::error(error)).discard()
            }
        }
    }

    /// Ends the setup wizard, whether it was finished or skipped. The choice is remembered, so the wizard does not show again
    /// while no account is stored. With an account stored the launcher opens; without one the sign-in page does.
    pub(super) fn setup_finish(&mut self) -> Task<LodestoneAppMessage> {
        self.setup_finished = true;
        let save = Task::future(async {
            match tokio::task::spawn_blocking(|| Preferences::update(|preferences| preferences.setup_finished = true)).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => warn!("Saving the setup choice failed: {error}"),
                Err(error) => warn!("Saving the setup choice failed: {error}"),
            }
        })
        .discard();
        if self.account.is_some() {
            Task::batch([save, self.enter_launcher()])
        } else {
            self.router.replace(Page::Login);
            save
        }
    }
}

#[cfg(test)]
#[path = "../../tests/app/setup.rs"]
mod tests;
