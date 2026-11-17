//! The instance detail page — the immersive per-instance view from
//! `04-instance.png`.
//!
//! The page is a fixed masthead plus tab row over one content region. Each tab
//! lives in its own module with its own state, message and view; this file
//! routes messages and lays out the shell. The instance model's own data (name,
//! loader, version, playtime, biome) comes from the database. Everything else
//! the tabs show is read from the instance folder by [`files`], on the blocking
//! pool, and applied when it lands.

pub mod browser;
pub mod content;
pub mod files;
pub(crate) mod format;
mod hero;
pub(crate) mod logs;
pub mod mods;
pub mod packs;
mod pictures;
pub mod screenshots;
pub mod settings;
mod tab;
pub mod versions;
mod view;
mod watch;
pub mod world_menu;
pub mod worlds;

use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::widget::container;
use iced::widget::image::Handle;
use iced::{Element, Fill, Padding, Size, Subscription, Task, alignment, event, window};

use crate::app::launch_settings::LaunchDefaults;
use crate::app::pages::Page;
use crate::app::pages::library::instance::Instance;
use crate::app::pages::library::instance_menu;
use files::{InstanceFiles, ModFile, WorldFile};
use watch::{Change, Update};

/// The page's horizontal gutter — 28px, per the design.
const GUTTER: f32 = 28.0;

pub use tab::Tab;

/// Messages the instance page handles.
#[derive(Debug, Clone)]
pub enum Message {
    /// Leaves the detail view — the root steps the router back.
    Back,
    /// Selects a tab.
    TabSelected(Tab),
    /// Reveals the instance folder (the root owns IO).
    RevealFolder,
    /// Opens the page the instance was installed from (the root owns IO).
    OpenSource,
    /// A row of the ⋯ menu was picked. A separator sends `None`; the root carries the action out.
    Menu(Option<instance_menu::Action>),
    /// Starts the instance's game. The root launches it.
    Launch,
    /// Stops the instance's game. The root stops it.
    Kill,
    /// Starts the instance's runs again. The root restarts them.
    Restart,
    /// Starts another process of an instance, from the button beside its running game. The root launches
    /// it.
    LaunchNew(String),
    /// The Mods tab's Add was pressed. The root opens Discover on the mods of this instance.
    AddMods,
    /// The padlock on the masthead was pressed for the locked instance `String`. The root asks before it unlocks the instance.
    Unlock(String),
    /// The open padlock on the masthead was pressed for the unlocked instance `String`. The root locks it at once.
    Lock(String),
    /// Requests navigation — the root intercepts this before the page's
    /// `update` runs (the `<Link>` of this framework).
    Navigate(Page),
    Mods(mods::Message),
    /// A message of the Files tab. The root carries out its requests before the page sees them.
    Browser(browser::Message),
    /// The Content tab's switch between its mods and its packs.
    Content(content::Message),
    /// The resource packs and shader packs of the Content tab. The root carries out the copies and removals.
    Packs(packs::Message),
    Worlds(worlds::Message),
    Screenshots(screenshots::Message),
    Logs(logs::Message),
    Settings(settings::Message),
    /// A message of the Versions tab. The root carries out the moves and the records before the page sees them.
    Versions(versions::Message),
    /// The instance folder was read. Stale when `generation` has moved on.
    Scanned {
        generation: u64,
        files: InstanceFiles,
    },
    /// A capture's thumbnail was decoded, or could not be (`None`).
    Thumbnail {
        generation: u64,
        path: PathBuf,
        handle: Option<Handle>,
    },
    /// The watch saw saves or captures change in the instance folder. Stale when `folder` is no longer the
    /// one shown.
    FilesChanged {
        folder: PathBuf,
        changes: Vec<Change>,
    },
    /// The changed saves and captures were read back. Stale when `generation` has moved on.
    FilesRead {
        generation: u64,
        updates: Vec<Update>,
    },
    /// The log refresh timer fired while the Logs tab is following.
    LogsTick,
    /// The log was re-read.
    LogsLoaded {
        generation: u64,
        lines: Vec<logs::LogLine>,
    },
}

/// The instance detail page state.
#[derive(Debug, Default)]
pub struct InstancePage {
    instance: Option<Instance>,
    /// The instance folder, which holds the files the tabs read.
    folder: PathBuf,
    tab: Tab,
    /// Bumped by every [`InstancePage::show`]. Results from an older generation
    /// belong to an instance the page no longer shows, so they are dropped.
    generation: u64,
    /// What the folder held at the last scan; `None` until that scan lands.
    files: Option<InstanceFiles>,
    mods: mods::State,
    /// The Files tab: the folder on screen, and what is selected in it.
    browser: browser::State,
    /// The section the Content tab shows.
    content_section: content::Section,
    packs: packs::State,
    worlds: worlds::State,
    screenshots: screenshots::State,
    logs: logs::State,
    settings: settings::State,
    versions: versions::State,
    /// The run whose console the Logs tab shows, when the runs panel opened one. `None` shows the
    /// newest run.
    console_run: Option<u64>,
    /// The changes the watch reported that no read has picked up yet.
    watch_pending: Vec<Change>,
    /// Whether a read of the changed saves and captures is on the blocking pool.
    reading: bool,
}

impl InstancePage {
    /// Shows the console of the run `serial` on the Logs tab, or the newest run's when `None`.
    pub fn focus_console(&mut self, serial: Option<u64>) {
        self.console_run = serial;
    }

    /// The run whose console the Logs tab was asked to show, if any.
    pub fn focused_run(&self) -> Option<u64> {
        self.console_run
    }

    /// Points the page at `instance`, whose files live in `folder`, with the launch `defaults` the instance inherits from.
    /// Resets the transient controls and every tab, and returns the task that reads the folder.
    pub fn show(&mut self, instance: Instance, folder: PathBuf, defaults: &LaunchDefaults) -> Task<Message> {
        self.generation += 1;
        let generation = self.generation;
        self.settings = settings::State::default();
        self.settings.sync(&instance, defaults);
        self.instance = Some(instance);
        self.folder = folder.clone();
        self.files = None;
        self.mods = mods::State::default();
        let locked = self.instance.as_ref().is_some_and(|shown| shown.locked);
        self.mods.set_locked(locked);
        // The files copied or cut stay on the launcher's clipboard, so they can be pasted into another instance.
        let clip = self.browser.clip().cloned();
        self.browser = browser::State::new(self.folder.clone(), locked);
        self.browser.set_clip(clip);
        self.content_section = content::Section::default();
        self.packs = packs::State::default();
        self.worlds = worlds::State::default();
        self.screenshots = screenshots::State::default();
        self.logs = logs::State::default();
        self.versions = versions::State::default();
        self.tab = Tab::Content;
        self.console_run = None;
        self.watch_pending.clear();
        self.reading = false;

        Task::perform(
            async move { tokio::task::spawn_blocking(move || files::scan(&folder)).await.unwrap_or_default() },
            move |files| Message::Scanned { generation, files },
        )
    }

    /// Replaces the shown instance's copy with a reload of it, so its state, playtime and settings are current, and
    /// resolves its launch settings against `defaults` again. A reload of any other instance is ignored.
    pub fn refresh(&mut self, instance: Instance, defaults: &LaunchDefaults) {
        if self.instance.as_ref().is_some_and(|shown| shown.id == instance.id) {
            self.settings.sync(&instance, defaults);
            self.versions.refresh(&instance);
            self.mods.set_locked(instance.locked);
            self.browser.set_read_only(instance.locked);
            self.instance = Some(instance);
        }
    }

    /// The release the Versions tab lists, for the root to read when it prepares an update.
    pub fn versions(&self) -> &versions::State {
        &self.versions
    }

    /// The Versions tab's state, for the root to mark the release an update is prepared for.
    pub fn versions_mut(&mut self) -> &mut versions::State {
        &mut self.versions
    }

    /// Hands the mods the folder holds now to the Mods tab, and to the count the masthead shows.
    pub fn set_mods(&mut self, mods: Vec<ModFile>) {
        if let Some(files) = &mut self.files {
            files.mods = mods.clone();
        }
        self.mods.set_mods(mods);
    }

    /// The instance the page shows, if one has been shown.
    pub fn instance(&self) -> Option<&Instance> {
        self.instance.as_ref()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TabSelected(tab) => {
                self.tab = tab;
                match &self.instance {
                    Some(_) if tab == Tab::Files => self.browser.open().map(Message::Browser),
                    Some(instance) if tab == Tab::Settings => self.settings.open(instance).map(Message::Settings),
                    Some(instance) if tab == Tab::Versions => self.versions.open(instance).map(Message::Versions),
                    _ => Task::none(),
                }
            }
            Message::Versions(message) => self.versions.update(message).map(Message::Versions),
            Message::Mods(message) => self.mods.update(message).map(Message::Mods),
            Message::Browser(message) => self.browser.update(message).map(Message::Browser),
            Message::Content(content::Message::Section(section)) => {
                self.content_section = section;
                Task::none()
            }
            // The root keeps the packs' state and carries out their actions before the page sees them.
            Message::Packs(_) => Task::none(),
            Message::Worlds(worlds::Message::Rescanned { generation, worlds }) if generation == self.generation => self.rescanned_worlds(worlds),
            Message::Worlds(message) => {
                self.worlds.update(message);
                Task::none()
            }
            Message::Screenshots(message) => self.update_screenshots(message),
            Message::FilesChanged { folder, changes } if folder == self.folder => self.files_changed(changes),
            Message::FilesRead { generation, updates } if generation == self.generation => self.files_read(updates),
            Message::Logs(message) => {
                self.logs.update(message);
                Task::none()
            }
            Message::Settings(message) => match &self.instance {
                Some(instance) => self.settings.update(message, instance).map(Message::Settings),
                None => Task::none(),
            },
            Message::Scanned { generation, files } if generation == self.generation => self.apply(files),
            Message::Thumbnail { generation, path, handle } if generation == self.generation => {
                self.screenshots.store(path, handle);
                self.decode_next_thumbnail()
            }
            Message::LogsTick if self.tab == Tab::Logs && self.logs.following() => self.read_logs(),
            Message::LogsLoaded { generation, lines } if generation == self.generation => {
                self.logs.set_lines(lines);
                Task::none()
            }
            // Navigation is intercepted by the root; the remaining actions
            // are display-only until the instance manager lands.
            Message::Back
            | Message::RevealFolder
            | Message::OpenSource
            | Message::Menu(_)
            | Message::Navigate(_)
            | Message::Launch
            | Message::Kill
            | Message::Restart
            | Message::LaunchNew(_)
            | Message::AddMods
            | Message::Unlock(_)
            | Message::Lock(_) => Task::none(),
            // Results for an instance the page no longer shows, and ticks the
            // Logs tab no longer needs.
            Message::Scanned { .. }
            | Message::Thumbnail { .. }
            | Message::FilesChanged { .. }
            | Message::FilesRead { .. }
            | Message::LogsTick
            | Message::LogsLoaded { .. } => Task::none(),
        }
    }

    /// The log refresh, and the viewer's keys. While the Logs tab is following, the log is re-read every
    /// two seconds. The viewer's keys wait while an alert is open, since the alert answers them. The
    /// root only adds this while the detail page is on screen.
    pub fn subscription(&self, alert_open: bool, dialog_open: bool) -> Subscription<Message> {
        let logs = if self.tab == Tab::Logs && self.logs.following() {
            iced::time::every(Duration::from_secs(2)).map(|_| Message::LogsTick)
        } else {
            Subscription::none()
        };
        let keys = if self.screenshots.viewing().is_some() && !alert_open {
            event::listen_with(viewer_key)
        } else {
            Subscription::none()
        };
        // The watch follows the instance's folders for as long as the page shows an instance.
        let watch = if self.instance.is_some() {
            Subscription::run_with(self.folder.clone(), watch::subscription)
        } else {
            Subscription::none()
        };
        // The Files tab follows its folder, its keys and its drops only while it is the tab on screen. A dialog over the
        // page has the keys and the drops while it is open.
        let browser = if self.tab == Tab::Files && self.instance.is_some() {
            let blocked = alert_open || dialog_open || self.viewer_open();
            self.browser.subscription(blocked).map(Message::Browser)
        } else {
            Subscription::none()
        };
        Subscription::batch([logs, keys, watch, browser])
    }

    /// The save at `path` in the instance's copy of its worlds, once the folder has been read.
    pub fn world(&self, path: &Path) -> Option<&WorldFile> {
        self.files.as_ref()?.worlds.iter().find(|world| world.path.as_path() == path)
    }

    /// The viewer over the window, while it is open.
    pub fn viewer(&self, window: Size) -> Option<Element<'_, Message>> {
        if let Some(dialog) = self.mods.dialog_view() {
            return Some(dialog.map(Message::Mods));
        }
        if let Some(dialog) = self.packs.dialog_view() {
            return Some(dialog.map(Message::Packs));
        }
        if let Some(dialog) = self.worlds.dialog_view() {
            return Some(dialog.map(Message::Worlds));
        }
        if self.tab == Tab::Settings
            && let Some(overlay) = self.settings.overlay()
        {
            return Some(overlay.map(Message::Settings));
        }
        self.screenshots.overlay(window).map(|overlay| overlay.map(Message::Screenshots))
    }

    /// The generation of the instance the page shows. A result that comes back for an older one is dropped.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The tab the page shows.
    pub fn tab(&self) -> Tab {
        self.tab
    }

    /// Whether one of the page's own dialogs is open over it.
    fn viewer_open(&self) -> bool {
        self.mods.dialog().is_some() || self.packs.dialog_view().is_some() || self.worlds.dialog_view().is_some()
    }

    /// The Files tab's state, which the root reads to carry out what the tab asks for.
    pub fn browser(&self) -> &browser::State {
        &self.browser
    }

    /// The Files tab's state, for the root to update as the disk answers.
    pub fn browser_mut(&mut self) -> &mut browser::State {
        &mut self.browser
    }

    /// The Mods tab's state, which the root reads and fills in as the platforms answer.
    pub fn mods(&self) -> &mods::State {
        &self.mods
    }

    /// The Mods tab's state, for the root to fill in as the platforms answer.
    pub fn mods_mut(&mut self) -> &mut mods::State {
        &mut self.mods
    }

    /// The packs of the Content tab, for the root to keep up to date.
    pub fn packs_mut(&mut self) -> &mut packs::State {
        &mut self.packs
    }

    /// The packs of the Content tab, as the page shows them.
    pub fn packs(&self) -> &packs::State {
        &self.packs
    }

    /// The Worlds tab's state, which the root reads for the shared-worlds dialog.
    pub fn worlds(&self) -> &worlds::State {
        &self.worlds
    }

    /// Handles the Screenshots tab's messages the page owns. The root carries out the rest: the
    /// actions and their results, which reach it first.
    fn update_screenshots(&mut self, message: screenshots::Message) -> Task<Message> {
        match message {
            screenshots::Message::Show(index) => {
                self.screenshots.show(index);
                self.decode_viewer()
            }
            screenshots::Message::Move(movement) => {
                self.screenshots.move_viewer(movement);
                self.decode_viewer()
            }
            screenshots::Message::Close => {
                self.screenshots.close();
                Task::none()
            }
            screenshots::Message::Key(key) => {
                let action = match key {
                    screenshots::Key::Delete => screenshots::Action::Delete,
                    screenshots::Key::Copy => screenshots::Action::Copy,
                };
                match self.screenshots.current_path() {
                    Some(path) => Task::done(Message::Screenshots(screenshots::Message::Menu { path, action: Some(action) })),
                    None => Task::none(),
                }
            }
            screenshots::Message::Full { generation, path, handle } if generation == self.generation => {
                self.screenshots.store_full(path, handle);
                Task::none()
            }
            // A decode for an instance the page no longer shows, and actions the root carries out.
            screenshots::Message::Full { .. }
            | screenshots::Message::Menu { .. }
            | screenshots::Message::DeleteAnswered { .. }
            | screenshots::Message::Deleted { .. }
            | screenshots::Message::Copied(_)
            | screenshots::Message::Opened(_) => Task::none(),
        }
    }

    /// Starts the full-size decodes the viewer still needs, on the blocking pool. Each lands as a `Full`.
    fn decode_viewer(&mut self) -> Task<Message> {
        let generation = self.generation;
        Task::batch(self.screenshots.begin_decodes().into_iter().map(|path| {
            let decode = path.clone();
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || files::decode_image(&decode, Some(screenshots::FULL)))
                        .await
                        .ok()
                        .flatten()
                },
                move |handle| Message::Screenshots(screenshots::Message::Full { generation, path, handle }),
            )
        }))
    }

    /// Reads the instance's saves again on the blocking pool, after a world was imported or backed up.
    pub fn rescan_worlds(&self) -> Task<Message> {
        let generation = self.generation;
        let folder = self.folder.clone();
        Task::perform(
            async move { tokio::task::spawn_blocking(move || files::scan_worlds(&folder)).await.unwrap_or_default() },
            move |worlds| Message::Worlds(worlds::Message::Rescanned { generation, worlds }),
        )
    }

    /// Takes a reread of the saves: the Worlds tab and the instance's own copy of its worlds both take it.
    fn rescanned_worlds(&mut self, worlds: Vec<WorldFile>) -> Task<Message> {
        if let Some(files) = &mut self.files {
            files.worlds = worlds.clone();
        }
        self.worlds.set_worlds(worlds);
        Task::none()
    }

    /// Drops the save in `path` from the Worlds tab, and from the instance's own copy of its worlds.
    pub fn remove_world(&mut self, path: &Path) {
        self.worlds.remove(path);
        if let Some(files) = &mut self.files {
            files.worlds.retain(|world| world.path.as_path() != path);
        }
    }

    /// Drops the capture in `path` from the Screenshots tab, and from the instance's own copy of its captures.
    /// The viewer may now show a neighbour whose full-size image is not decoded yet, so those start.
    pub fn remove_screenshot(&mut self, path: &Path) -> Task<Message> {
        self.screenshots.remove(path);
        if let Some(files) = &mut self.files {
            files.screenshots.retain(|shot| shot.path.as_path() != path);
        }
        self.decode_viewer()
    }

    /// Hands a scan's results to the tabs and starts the thumbnails.
    fn apply(&mut self, files: InstanceFiles) -> Task<Message> {
        self.packs.set_packs(files.resource_packs.clone(), files.shader_packs.clone());
        self.mods.set_mods(files.mods.clone());
        self.worlds.set_worlds(files.worlds.clone());
        self.worlds.set_servers(files.servers.clone());
        self.screenshots.set_shots(files.screenshots.clone());
        self.logs.set_lines(files.log.clone());
        self.files = Some(files);
        // Changes the watch reported while the scan ran are not in the scan, so they are read now.
        Task::batch([self.read_pending(), self.decode_next_thumbnail()])
    }

    /// Takes a batch of changes from the watch. They are read back at once unless a read is already running,
    /// which picks them up when it lands.
    fn files_changed(&mut self, changes: Vec<Change>) -> Task<Message> {
        self.watch_pending.extend(changes);
        self.read_pending()
    }

    /// Reads the pending changes on the blocking pool, unless a read is running or the first scan has not
    /// landed. Changes that arrive before the scan lands are read once it has.
    fn read_pending(&mut self) -> Task<Message> {
        if self.reading || self.watch_pending.is_empty() || self.files.is_none() {
            return Task::none();
        }
        let changes = watch::coalesce(std::mem::take(&mut self.watch_pending));
        self.reading = true;
        let generation = self.generation;
        let folder = self.folder.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || watch::read(&folder, changes))
                    .await
                    .unwrap_or_default()
            },
            move |updates| Message::FilesRead { generation, updates },
        )
    }

    /// Merges what a read found into the instance's copy of its saves, captures, mods, packs and servers, then hands each list
    /// to its tab. A save or capture the watch did not report keeps its state, and so keeps its thumbnail.
    fn files_read(&mut self, updates: Vec<Update>) -> Task<Message> {
        self.reading = false;
        if let Some(files) = &mut self.files {
            let mut mods_changed = false;
            let mut packs_changed = false;
            let mut servers_changed = false;
            for update in updates {
                match update {
                    Update::Worlds(worlds) => files.worlds = worlds,
                    Update::World(path, world) => {
                        files.worlds.retain(|existing| existing.path != path);
                        files.worlds.extend(world);
                    }
                    Update::Shots(shots) => files.screenshots = shots,
                    Update::Shot(path, shot) => {
                        files.screenshots.retain(|existing| existing.path != path);
                        files.screenshots.extend(shot);
                    }
                    Update::Mods(mods) => {
                        files.mods = mods;
                        mods_changed = true;
                    }
                    Update::Packs(resource, shader) => {
                        files.resource_packs = resource;
                        files.shader_packs = shader;
                        packs_changed = true;
                    }
                    Update::Servers(servers) => {
                        files.servers = servers;
                        servers_changed = true;
                    }
                }
            }
            files.worlds.sort_by_key(|world| std::cmp::Reverse(world.saved));
            files.screenshots.sort_by_key(|shot| std::cmp::Reverse(shot.modified));
            self.worlds.set_worlds(files.worlds.clone());
            self.screenshots.set_shots(files.screenshots.clone());
            if mods_changed {
                self.mods.set_mods(files.mods.clone());
            }
            if packs_changed {
                self.packs.set_packs(files.resource_packs.clone(), files.shader_packs.clone());
            }
            if servers_changed {
                self.worlds.set_servers(files.servers.clone());
            }
        }
        Task::batch([self.read_pending(), self.decode_viewer(), self.decode_next_thumbnail()])
    }

    /// Decodes the next pending thumbnail on the blocking pool. One at a time:
    /// each result starts the next, so a folder full of captures never holds
    /// more than one full-size image in memory.
    fn decode_next_thumbnail(&mut self) -> Task<Message> {
        let Some(path) = self.screenshots.next_pending() else {
            return Task::none();
        };
        let generation = self.generation;
        let decode = path.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || files::decode_image(&decode, Some(screenshots::THUMBNAIL)))
                    .await
                    .ok()
                    .flatten()
            },
            move |handle| Message::Thumbnail { generation, path, handle },
        )
    }

    /// Re-reads the instance's log on the blocking pool.
    fn read_logs(&self) -> Task<Message> {
        let generation = self.generation;
        let folder = self.folder.clone();
        Task::perform(
            async move { tokio::task::spawn_blocking(move || files::read_log(&folder)).await.unwrap_or_default() },
            move |lines| Message::LogsLoaded { generation, lines },
        )
    }
}

/// The viewer's key presses, as the page's messages.
fn viewer_key(input: iced::Event, status: event::Status, id: window::Id) -> Option<Message> {
    screenshots::key_event(input, status, id).map(Message::Screenshots)
}

/// Positions `content` inside a box that fills its parent: the padding and
/// alignment pin the content to one corner or edge.
fn layer<'a, M: 'a>(content: Element<'a, M>, align_x: alignment::Horizontal, align_y: alignment::Vertical, padding: Padding) -> Element<'a, M> {
    container(content)
        .width(Fill)
        .height(Fill)
        .align_x(align_x)
        .align_y(align_y)
        .padding(padding)
        .into()
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/instance/mod.rs"]
mod tests;
