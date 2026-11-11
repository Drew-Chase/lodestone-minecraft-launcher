//! The Mods tab's actions the root carries out: opening a mod's resource page, checking the mods against their platforms,
//! deleting a mod, switching or updating its release, linking it to a project, and listing its dependencies. The tab asks
//! for them, and what the platforms and the disk answer comes back here as [`Message`]s. The alerts, the trash and the
//! database belong to the root.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use iced::Task;
use iced::widget::image;
use lodestone_platforms::{ContentType, ContentVersion, SearchPage, SearchQuery, SearchResult, VersionDependency};
use tracing::warn;

use super::LodestoneAppMessage;
use super::LodestoneLauncher;
use super::components::alert::Alert;
use super::database::Database;
use super::discover::{MODRINTH, curseforge, details_of, versions_of};
use super::install::mods::{add_to_instance, fits};
use super::lang;
use super::mod_sources::{CheckContext, Checks, check_mods, sha1_of_file};
use super::pages::Page;
use super::pages::discover::{AddRequest, InstanceChoice, Project, Source, load_image};
use super::pages::instance::Tab;
use super::pages::instance::files;
use super::pages::instance::files::ModFile;
use super::pages::instance::mods::{Action, Associate, Dependencies, Dialog, DialogMessage, Line, Picked, Switch, project_key, scroll_to_release};
use super::pages::resource::{Listing, Load};
use super::reveal::reveal_file;
use super::shared_library::ModSource;
mod delete;
mod deps;
mod link;
mod replace;
mod toggle;

pub(crate) use replace::replace_jar;

/// How many hits a search for a link asks a platform for.
const SEARCH_LIMIT: u32 = 20;
/// How long a full check of an instance's mods is trusted before the platforms are asked again.
const MOD_CHECK_TTL: Duration = Duration::from_secs(10 * 60);
/// The least time between two checks of the Mods tab's mods against their platforms, across the whole launcher. The
/// platforms rate-limit their answers, so a check asked for sooner waits until this has passed.
const MOD_CHECK_INTERVAL: Duration = Duration::from_secs(120);

/// When an instance's mods were last checked in full, and the newer release each checked mod has, by file name. Kept for
/// the session, so reopening the Mods tab shows the updates it found without asking the platforms again.
#[derive(Debug)]
pub struct ModCheck {
    pub at: Instant,
    pub updates: HashMap<String, ContentVersion>,
}

/// What the platforms and the disk answered for the Mods tab. Each answer carries the `generation` of the instance it
/// was asked about, and an answer for an instance the page no longer shows is dropped.
#[derive(Debug, Clone)]
pub enum Message {
    /// The instance's recorded sources were read.
    Sources {
        generation: u64,
        result: Result<Vec<ModSource>, String>,
    },
    /// The mods were checked against their platforms. A `full` check looked at every mod, and the others at the ones named.
    Checked {
        generation: u64,
        full: bool,
        result: Result<Checks, String>,
    },
    /// The mods folder was read again after a change to it.
    Rescanned { generation: u64, mods: Vec<ModFile> },
    /// The question about deleting a mod was answered.
    DeleteAnswered { generation: u64, file_name: String, confirmed: bool },
    /// A mod's jar was moved to the trash, or could not be.
    Deleted {
        generation: u64,
        file_name: String,
        result: Result<(), String>,
    },
    /// A mod's jar was replaced by the release chosen, or could not be. The records name the new jar first.
    Replaced {
        generation: u64,
        file_name: String,
        result: Result<Vec<ModSource>, String>,
    },
    /// The releases a switch can install, or the error reading them.
    Releases {
        generation: u64,
        file_name: String,
        result: Result<Vec<ContentVersion>, String>,
    },
    /// The hits of a search for a link, or the error.
    Searched {
        generation: u64,
        source: Source,
        query: String,
        result: Result<SearchPage, String>,
    },
    /// The releases of the project picked for a link, or the error reading them.
    Picked {
        generation: u64,
        project: SearchResult,
        result: Result<PickedReleases, String>,
    },
    /// The link was recorded, or could not be.
    Linked {
        generation: u64,
        file_name: String,
        result: Result<ModSource, String>,
    },
    /// The icon of a search hit was loaded from `url`, or could not be.
    Icon {
        generation: u64,
        url: String,
        handle: Option<image::Handle>,
    },
    /// The picture of a linked mod's platform project was loaded under `key`, or the project has none.
    ProjectIcon {
        generation: u64,
        key: String,
        handle: Option<image::Handle>,
    },
    /// The dependencies of a mod's release, or the error reading them.
    Dependencies {
        generation: u64,
        file_name: String,
        result: Result<Vec<Line>, String>,
    },
    /// A mod's jar was renamed for its switch, or could not be. `result` is the name the jar has now.
    Toggled {
        generation: u64,
        file_name: String,
        enabled: bool,
        result: Result<String, String>,
    },
    /// The wait for the next check of the Mods tab's mods, which came too soon after the last one, has ended.
    ChecksDue,
}

/// The releases of a project picked for a link, the one that fits the jar best, and the jar's SHA-1.
#[derive(Debug, Clone)]
pub struct PickedReleases {
    pub releases: Vec<ContentVersion>,
    pub chosen: Option<usize>,
    pub jar_sha1: Option<String>,
}

/// What a task about the shown instance needs: the instance, and its folder, which holds the `mods/` folder.
#[derive(Debug, Clone)]
struct Context {
    instance: InstanceChoice,
    folder: PathBuf,
}

impl Context {
    fn mods_dir(&self) -> PathBuf {
        self.folder.join("mods")
    }

    fn check(&self) -> CheckContext {
        CheckContext {
            instance: self.instance.id.clone(),
            folder: self.folder.clone(),
            loader: self.instance.loader.to_db().to_string(),
            minecraft: self.instance.mc_version.clone(),
        }
    }
}

impl LodestoneLauncher {
    /// The context of the instance the Mods tab shows.
    fn mods_context(&self) -> Option<Context> {
        let instance = self.instance_page.instance()?;
        Some(Context {
            instance: InstanceChoice::of(instance),
            folder: self.paths.instance(&instance.directory_name),
        })
    }

    /// Opens a resource page from the Mods tab, on the instance the tab shows when one is shown.
    pub(super) fn open_from_mods_tab(&mut self, listing: Listing) -> Task<LodestoneAppMessage> {
        match self.instance_page.instance().map(InstanceChoice::of) {
            Some(instance) => self.open_resource_from_instance(listing, instance),
            None => self.open_resource(listing, None),
        }
    }

    /// Whether `generation` is the instance the Mods tab shows now.
    fn mods_fresh(&self, generation: u64) -> bool {
        generation == self.instance_page.generation()
    }

    /// Reads the mods and the packs folders again, and the instances the library counts their mods by, when history brings
    /// the instance page back. The watch runs only while the page is on screen, so a jar or pack added while another page
    /// showed is not in the lists until this read.
    pub(super) fn rescan_on_return(&mut self) -> Task<LodestoneAppMessage> {
        if self.router.current() == Page::Instance {
            Task::batch([self.rescan_mods(), self.reread_packs(), self.load_instances()])
        } else {
            Task::none()
        }
    }

    /// The mods were read. Their recorded sources are loaded, and the checks against the platforms start once the Mods
    /// tab is open.
    pub(super) fn mods_scanned(&mut self, generation: u64) -> Task<LodestoneAppMessage> {
        let (Some(context), Some(db)) = (self.mods_context(), self.db.clone()) else {
            return Task::none();
        };
        let instance = context.instance.id.clone();
        let read = Task::perform(async move { db.mod_sources_of(&instance).await }, move |result| {
            LodestoneAppMessage::ModActions(Message::Sources { generation, result })
        });
        Task::batch([read, self.mods_checks_due()])
    }

    /// Checks the shown instance's mods against their platforms, when the Mods tab is open and the folder has been read.
    /// Entering the tab asks for a check each time. The launcher never starts two checks less than [`MOD_CHECK_INTERVAL`]
    /// apart, so a check that comes too soon waits in a timer for the interval to pass, and then runs. Several requests in
    /// that wait share one timer.
    pub(super) fn mods_checks_due(&mut self) -> Task<LodestoneAppMessage> {
        let shown = self.instance_page.mods();
        if self.instance_page.tab() != Tab::Content || shown.checking() || !shown.has_mods() {
            return Task::none();
        }
        if let Some(started) = self.mod_check_started {
            let elapsed = started.elapsed();
            if elapsed < MOD_CHECK_INTERVAL {
                if self.mod_check_timer {
                    return Task::none();
                }
                self.mod_check_timer = true;
                return Task::perform(tokio::time::sleep(MOD_CHECK_INTERVAL - elapsed), |()| {
                    LodestoneAppMessage::ModActions(Message::ChecksDue)
                });
            }
        }
        self.start_mod_checks()
    }

    /// Starts a check of the shown instance's mods against their platforms now. A check with a full answer from the last
    /// [`MOD_CHECK_TTL`] only looks up the jars with no record, and shows the updates it found before. Otherwise every
    /// mod's update is checked.
    fn start_mod_checks(&mut self) -> Task<LodestoneAppMessage> {
        let (Some(context), Some(db)) = (self.mods_context(), self.db.clone()) else {
            return Task::none();
        };
        let generation = self.instance_page.generation();
        let names = self.instance_page.mods().file_names();
        self.mod_check_started = Some(Instant::now());
        self.instance_page.mods_mut().set_checking(true);
        match self
            .mod_checks
            .get(&context.instance.id)
            .filter(|check| check.at.elapsed() < MOD_CHECK_TTL)
        {
            Some(check) => {
                let updates = check.updates.clone();
                self.instance_page.mods_mut().apply_checks(Vec::new(), updates, &names);
                Task::perform(check_mods(db, context.check(), names, Some(Vec::new())), move |result| {
                    LodestoneAppMessage::ModActions(Message::Checked {
                        generation,
                        full: false,
                        result,
                    })
                })
            }
            None => Task::perform(check_mods(db, context.check(), names, None), move |result| {
                LodestoneAppMessage::ModActions(Message::Checked {
                    generation,
                    full: true,
                    result,
                })
            }),
        }
    }

    /// Applies a check to the Mods tab, and remembers the updates it found for the session. A full check restarts the time
    /// the session trusts it for. A check of single mods only updates a session that has a full check to remember.
    fn apply_checks(&mut self, full: bool, checks: Checks) {
        let Some(instance) = self.instance_page.instance().map(|instance| instance.id.clone()) else {
            return;
        };
        let Checks {
            identified,
            updates,
            checked,
        } = checks;
        if full || self.mod_checks.contains_key(&instance) {
            let remembered = self.mod_checks.entry(instance).or_insert_with(|| ModCheck {
                at: Instant::now(),
                updates: HashMap::new(),
            });
            for name in &checked {
                match updates.get(name) {
                    Some(release) => remembered.updates.insert(name.clone(), release.clone()),
                    None => remembered.updates.remove(name),
                };
            }
            if full {
                remembered.at = Instant::now();
            }
        }
        self.instance_page.mods_mut().apply_checks(identified, updates, &checked);
    }

    /// Applies what came back for the Mods tab. An answer for an instance the page no longer shows is dropped.
    pub(super) fn mod_actions_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::Sources { generation, result } if self.mods_fresh(generation) => {
                match result {
                    Ok(records) => self.instance_page.mods_mut().apply_sources(records),
                    Err(error) => warn!("reading the mods' sources failed: {error}"),
                }
                self.project_icons(generation)
            }
            Message::ProjectIcon { generation, key, handle } if self.mods_fresh(generation) => {
                if let Some(handle) = handle {
                    self.instance_page.mods_mut().set_project_icon(key, handle);
                }
                Task::none()
            }
            Message::Checked { generation, full, result } if self.mods_fresh(generation) => {
                self.instance_page.mods_mut().set_checking(false);
                match result {
                    Ok(checks) => {
                        // The jars just identified are saved, so the Discover cache takes them too.
                        self.discover.note_sources(&checks.identified);
                        self.apply_checks(full, checks)
                    }
                    Err(error) => warn!("checking the mods failed: {error}"),
                }
                Task::none()
            }
            Message::Rescanned { generation, mods } if self.mods_fresh(generation) => {
                self.instance_page.set_mods(mods);
                Task::none()
            }
            Message::DeleteAnswered {
                generation,
                file_name,
                confirmed,
            } if self.mods_fresh(generation) => self.answer_mod_delete(file_name, confirmed),
            Message::Deleted {
                generation,
                file_name,
                result,
            } if self.mods_fresh(generation) => self.mod_deleted(file_name, result),
            Message::Replaced {
                generation,
                file_name,
                result,
            } if self.mods_fresh(generation) => self.mod_replaced(file_name, result),
            Message::Releases {
                generation,
                file_name,
                result,
            } if self.mods_fresh(generation) => {
                self.mod_releases(file_name, result);
                Task::none()
            }
            Message::Searched {
                generation,
                source,
                query,
                result,
            } if self.mods_fresh(generation) => self.mod_searched(generation, source, query, result),
            Message::Icon { generation, url, handle } if self.mods_fresh(generation) => {
                self.mod_icon(url, handle);
                Task::none()
            }
            Message::Picked { generation, project, result } if self.mods_fresh(generation) => self.mod_picked(project, result),
            Message::Linked {
                generation,
                file_name,
                result,
            } if self.mods_fresh(generation) => self.mod_linked(file_name, result),
            Message::Dependencies {
                generation,
                file_name,
                result,
            } if self.mods_fresh(generation) => {
                self.mod_dependencies(file_name, result);
                Task::none()
            }
            Message::Toggled {
                generation,
                file_name,
                enabled,
                result,
            } if self.mods_fresh(generation) => self.mod_toggled(file_name, enabled, result),
            Message::ChecksDue => {
                self.mod_check_timer = false;
                self.mods_checks_due()
            }
            // An answer for an instance the page no longer shows.
            _ => Task::none(),
        }
    }

    /// A mod's row was pressed. A mod that came from a platform opens its resource page, and one that did not opens the
    /// search that links it.
    pub(super) fn mod_row_pressed(&mut self, file_name: String) -> Task<LodestoneAppMessage> {
        let Some(row) = self.instance_page.mods().row(&file_name) else {
            return Task::none();
        };
        match row.source.clone() {
            Some(record) => self.open_mod_resource(&record),
            None => self.open_link(file_name),
        }
    }

    /// Opens the resource page of the project a mod was installed from. The page returns to the instance whose Mods tab
    /// opened it.
    fn open_mod_resource(&mut self, record: &ModSource) -> Task<LodestoneAppMessage> {
        let Some(source) = Source::from_key(&record.platform) else {
            return Task::none();
        };
        self.open_from_mods_tab(Listing {
            source,
            content: ContentType::Mod,
            project_id: record.project_id.clone(),
            name: record.name.clone(),
            author: None,
            summary: String::new(),
            downloads: 0,
            icon_url: None,
            web_url: None,
            updated: None,
        })
    }

    /// Shows a mod's jar in the file browser, with the jar selected.
    fn reveal_mod(&self, file_name: &str) -> Task<LodestoneAppMessage> {
        match self.mods_context() {
            Some(context) => reveal_file(context.mods_dir().join(file_name)),
            None => Task::none(),
        }
    }

    /// A row of a mod's menu was picked. A separator carries no action.
    pub(super) fn mod_menu_action(&mut self, file_name: String, action: Option<Action>) -> Task<LodestoneAppMessage> {
        match action {
            Some(Action::Update) => match self.instance_page.mods().row(&file_name).and_then(|row| row.update.clone()) {
                Some(release) => self.replace_mod(file_name, release),
                None => Task::none(),
            },
            Some(Action::Switch) => self.open_switch(file_name),
            Some(Action::Dependencies) => self.open_dependencies(file_name),
            Some(Action::Dependents) => self.open_dependents(file_name),
            Some(Action::Associate) => self.open_link(file_name),
            Some(Action::Reveal) => self.reveal_mod(&file_name),
            Some(Action::Delete) => self.ask_mod_delete(file_name),
            None => Task::none(),
        }
    }
}

impl LodestoneLauncher {
    /// Asks the platforms for the pictures of the projects the linked mods came from. Only mods whose jars declare no
    /// picture ask, and each project is asked once.
    fn project_icons(&mut self, generation: u64) -> Task<LodestoneAppMessage> {
        let wanted = self.instance_page.mods_mut().take_icon_requests();
        Task::batch(wanted.into_iter().filter_map(|record| {
            let source = Source::from_key(&record.platform)?;
            let key = project_key(&record);
            Some(Task::perform(link::project_picture(source, record.project_id), move |handle| {
                LodestoneAppMessage::ModActions(Message::ProjectIcon { generation, key, handle })
            }))
        }))
    }

    /// The dialog's buttons that need the root: a search, a pick, a link, a switch, and a dependency's page. The rest the
    /// tab answers itself.
    pub(super) fn mod_dialog_action(&mut self, message: DialogMessage) -> Task<LodestoneAppMessage> {
        match message {
            DialogMessage::Search => self.search_from_dialog(),
            DialogMessage::Platform(source) => self.change_platform(source),
            DialogMessage::Pick(index) => self.pick_from_dialog(index),
            DialogMessage::Confirm => self.confirm_link(),
            DialogMessage::Use(index) => self.use_release(index),
            DialogMessage::Open(index) => self.open_dependency(index),
            DialogMessage::Close | DialogMessage::Query(_) | DialogMessage::Choose(_) | DialogMessage::Back => Task::none(),
        }
    }
}
