//! Settings tab: the instance's launch settings and its identity, as one page of sections. A sidebar names the sections,
//! jumps the page to one, and follows the page as it scrolls.
//!
//! The rows read the copy of the instance the root keeps, with the launch overrides resolved against the global
//! defaults. An edit that changes the instance leaves as [`Message::Edit`]: the root applies it, saves it, and hands the
//! saved copy back, so the page shows what is stored.

mod general;
mod nav;
mod sections;

use std::path::{Path, PathBuf};

use aurora_leaf::Icon;
use iced::widget::{Space, column, container, row};
use iced::{Color, Element, Fill, Padding, Task, alignment};
use lodestone_loaders::LoaderVersion;

use crate::app::components::instance_icon;
use crate::app::components::launch_rows::{self, Axis, EnvEdit, JavaOptions, ResolutionDraft, Shown};
use crate::app::components::scrollbar;
use crate::app::components::setting_row::section_header;
use crate::app::install::catalog::{self, BundledJava, VersionEntry};
use crate::app::install::java;
use crate::app::lang;
use crate::app::launch_settings::{EnvVar, InstanceSettings, LaunchChange, LaunchDefaults, LaunchSettings, Resolution};
use crate::app::pages::library::instance::{Biome, Instance, InstanceIcon, Loader};
use crate::app::pages::library::new_instance::banner::{self, SCENE_COUNT};
use crate::app::pages::library::new_instance::{BannerChoice, IconChoice, Upload, banner_editor, icon_editor, upload};
use crate::app::theme::color;

/// The sections of the page, in order. The sidebar lists them, and each one has an anchor the page scrolls to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Section {
    #[default]
    General,
    JavaMemory,
    Window,
    LaunchArgs,
    GameVersion,
    PathsFiles,
    Advanced,
    DangerZone,
}

impl Section {
    pub const ALL: [Section; 8] = [
        Section::General,
        Section::JavaMemory,
        Section::Window,
        Section::LaunchArgs,
        Section::GameVersion,
        Section::PathsFiles,
        Section::Advanced,
        Section::DangerZone,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Section::General => lang::t("instance.settings.section.general"),
            Section::JavaMemory => lang::t("instance.settings.section.java_memory"),
            Section::Window => lang::t("instance.settings.section.window"),
            Section::LaunchArgs => lang::t("instance.settings.section.launch_args"),
            Section::GameVersion => lang::t("instance.settings.section.game_version"),
            Section::PathsFiles => lang::t("instance.settings.section.paths_files"),
            Section::Advanced => lang::t("instance.settings.section.advanced"),
            Section::DangerZone => lang::t("instance.settings.section.danger_zone"),
        }
    }

    pub fn subtitle(self) -> &'static str {
        match self {
            Section::General => lang::t("instance.settings.subtitle.general"),
            Section::JavaMemory => lang::t("instance.settings.subtitle.java_memory"),
            Section::Window => lang::t("instance.settings.subtitle.window"),
            Section::LaunchArgs => lang::t("instance.settings.subtitle.launch_args"),
            Section::GameVersion => lang::t("instance.settings.subtitle.game_version"),
            Section::PathsFiles => lang::t("instance.settings.subtitle.paths_files"),
            Section::Advanced => lang::t("instance.settings.subtitle.advanced"),
            Section::DangerZone => lang::t("instance.settings.subtitle.danger_zone"),
        }
    }

    /// The Lucide glyph the section's tile and sidebar entry show.
    pub fn icon(self) -> &'static str {
        match self {
            Section::General => Icon::lucide().tag(),
            Section::JavaMemory => Icon::lucide().cpu(),
            Section::Window => Icon::lucide().image(),
            Section::LaunchArgs => Icon::lucide().terminal(),
            Section::GameVersion => Icon::lucide().upload(),
            Section::PathsFiles => Icon::lucide().folder(),
            Section::Advanced => Icon::lucide().settings(),
            Section::DangerZone => Icon::lucide().trash(),
        }
    }

    /// The accent the section's tile is drawn in.
    pub fn tint(self) -> Color {
        match self {
            Section::DangerZone => color::buttons::danger_background_hover(),
            _ => color::primary(),
        }
    }

    /// The id of the section's container, which the page measures and scrolls to.
    pub fn anchor_id(self) -> iced::widget::Id {
        iced::widget::Id::new(match self {
            Section::General => "instance-settings-general",
            Section::JavaMemory => "instance-settings-java-memory",
            Section::Window => "instance-settings-window",
            Section::LaunchArgs => "instance-settings-launch-args",
            Section::GameVersion => "instance-settings-game-version",
            Section::PathsFiles => "instance-settings-paths-files",
            Section::Advanced => "instance-settings-advanced",
            Section::DangerZone => "instance-settings-danger-zone",
        })
    }
}

/// A change to the instance. The page emits these, and the root applies each to its copy and saves the result.
#[derive(Debug, Clone)]
pub enum Edit {
    /// The name, as typed.
    Name(String),
    /// A launch setting, which the instance overrides.
    Launch(LaunchChange),
    /// Whether the Game Version section lists snapshots.
    Snapshots(bool),
    /// A Minecraft version was picked, by id.
    MinecraftVersion(String),
    /// A loader build was picked, by version.
    LoaderBuild(String),
    /// The banner the editor saved.
    Banner(BannerChoice),
    /// The banner picture an upload was written to, relative to the instance folder.
    BannerWritten(Result<PathBuf, String>),
    /// The icon the editor saved.
    Icon(IconChoice),
    /// The icon picture an upload was written to, relative to the instance folder.
    IconWritten(Result<PathBuf, String>),
    /// Every launch setting goes back to the global defaults. The root asks first.
    Reset,
    /// A picture could not be read or written. The root reports it.
    Failed(String),
}

/// Messages the Settings tab emits and handles.
#[derive(Debug, Clone)]
pub enum Message {
    /// A sidebar entry was pressed: the page scrolls to that section.
    Section(Section),
    /// The page scrolled. `at_end` is set when it is at its bottom with more above.
    Scrolled {
        offset: f32,
        at_end: bool,
    },
    /// Where each section's top sits in the page.
    Measured(Vec<(Section, f32)>),
    /// The width or height field was typed in.
    Resolution(Axis, String),
    OpenEnvironment,
    CloseEnvironment,
    /// One change to the environment variables.
    Environment(EnvEdit),
    OpenBannerEditor,
    BannerEditor(banner_editor::Message),
    /// The banner the instance shows now, read for the editor.
    BannerCurrent(Result<Vec<u8>, String>),
    /// The picture the banner dialog returned.
    BannerPicked(Result<Option<Vec<u8>>, String>),
    OpenIconEditor,
    IconEditor(icon_editor::Message),
    /// The icon the instance shows now, read for the editor.
    IconCurrent(Result<Vec<u8>, String>),
    /// The picture the icon dialog returned.
    IconPicked(Result<Option<Vec<u8>>, String>),
    /// The icons used most recently, for the icon editor.
    RecentsLoaded(Result<Vec<InstanceIcon>, String>),
    /// The runtimes the platform bundles.
    BundledLoaded(Result<Vec<BundledJava>, String>),
    /// The major version of the system Java, when one was found.
    SystemJavaLoaded(Option<u32>),
    /// The machine's memory, in whole gigabytes.
    MemoryLoaded(u32),
    /// Mojang's versions.
    VersionsLoaded(Result<Vec<VersionEntry>, String>),
    /// Asks for the loader builds of the instance's Minecraft version.
    LoadBuilds,
    /// The loader builds of Minecraft version `mc`.
    BuildsLoaded {
        mc: String,
        result: Result<Vec<LoaderVersion>, String>,
    },
    /// A change to the instance, which the root saves.
    Edit(Edit),
    /// Asks before the launch settings go back to the defaults. The root answers.
    ResetSettings,
    /// Queues a reinstall of the game's files. The root queues it.
    Reinstall,
    /// Opens the instance folder. The root opens it.
    RevealFolder,
    /// Deletes the instance after confirmation. The root asks.
    DeleteInstance,
}

/// The Settings tab's state: the section the sidebar shows, the lists the selects offer, and what is open on the page.
/// The rows show the instance's copy, resolved against the defaults by [`State::sync`].
#[derive(Debug, Default)]
pub struct State {
    section: Section,
    /// Where each section's top sits, measured on the last scroll, change or sidebar press.
    offsets: Vec<(Section, f32)>,
    /// The page's scroll offset, and whether it is at its bottom with more above.
    scroll: f32,
    at_end: bool,
    /// The launch values the rows show, with each override resolved against the defaults.
    resolved: Option<LaunchSettings>,
    /// The instance's own overrides, which tell the rows which settings are inherited.
    overrides: Option<InstanceSettings>,
    java: JavaOptions,
    /// The largest heap the slider offers, in gigabytes. Set from the machine's memory once the tab opens.
    ceiling_gb: u32,
    versions: Vec<VersionEntry>,
    versions_error: Option<String>,
    builds: Option<Vec<LoaderVersion>>,
    builds_error: Option<String>,
    recents: Vec<InstanceIcon>,
    /// The resolution fields while they hold text that is not yet a valid pair.
    draft: ResolutionDraft,
    environment_open: bool,
    banner: Option<banner_editor::BannerEditor>,
    icon: Option<icon_editor::IconEditor>,
}

impl State {
    /// Shows the instance's launch values, resolved against `defaults`. Called whenever the instance or the defaults change.
    pub fn sync(&mut self, instance: &Instance, defaults: &LaunchDefaults) {
        self.resolved = Some(instance.settings.resolve(defaults));
        self.overrides = Some(instance.settings.clone());
    }

    /// Loads what the tab's lists need: the runtimes, the system Java, the machine's memory, Mojang's versions and the
    /// loader builds. Called when the tab is selected.
    pub fn open(&self, instance: &Instance) -> Task<Message> {
        let mut tasks = vec![
            Task::perform(catalog::bundled_java(), Message::BundledLoaded),
            Task::perform(java::system(1), |found| Message::SystemJavaLoaded(found.map(|java| java.major))),
            Task::perform(
                async { tokio::task::spawn_blocking(launch_rows::machine_memory_gb).await.unwrap_or(16) },
                Message::MemoryLoaded,
            ),
            Task::perform(catalog::versions(), Message::VersionsLoaded),
        ];
        if instance.loader != Loader::Vanilla {
            tasks.push(builds_of(instance));
        }
        Task::batch(tasks)
    }

    /// Applies a message. A change to the instance leaves as [`Message::Edit`] for the root, and the rest is the page's
    /// own business.
    pub fn update(&mut self, message: Message, instance: &Instance) -> Task<Message> {
        match message {
            Message::Section(section) => {
                self.section = section;
                nav::scroll_to(section)
            }
            Message::Scrolled { offset, at_end } => {
                self.scroll = offset;
                self.at_end = at_end;
                nav::measure().map(Message::Measured)
            }
            Message::Measured(offsets) => {
                self.offsets = offsets;
                if let Some(section) = nav::active(&self.offsets, self.scroll, self.at_end) {
                    self.section = section;
                }
                Task::none()
            }
            Message::Resolution(axis, text) => self.resolution(axis, &text),
            Message::OpenEnvironment => {
                self.environment_open = true;
                Task::none()
            }
            Message::CloseEnvironment => {
                self.environment_open = false;
                Task::none()
            }
            Message::Environment(edit) => {
                let mut vars = self.resolved.as_ref().map(|resolved| resolved.env.clone()).unwrap_or_default();
                edit.apply(&mut vars);
                edit_task(Edit::Launch(LaunchChange::Env(vars)))
            }
            Message::OpenBannerEditor => self.open_banner(instance),
            Message::BannerEditor(message) => self.banner_message(message),
            Message::BannerCurrent(result) => {
                self.set_banner_picture(result);
                Task::none()
            }
            Message::BannerPicked(result) => match result {
                Ok(Some(png)) => {
                    if let Some(editor) = &mut self.banner {
                        editor.set_upload(Upload::new(png));
                    }
                    Task::none()
                }
                Ok(None) => Task::none(),
                Err(error) => failed(error),
            },
            Message::OpenIconEditor => self.open_icon(instance),
            Message::IconEditor(message) => self.icon_message(message),
            Message::IconCurrent(result) => {
                self.set_icon_picture(result);
                Task::none()
            }
            Message::IconPicked(result) => match result {
                Ok(Some(png)) => {
                    if let Some(editor) = &mut self.icon {
                        editor.set_upload(Upload::new(png));
                    }
                    Task::none()
                }
                Ok(None) => Task::none(),
                Err(error) => failed(error),
            },
            Message::RecentsLoaded(result) => {
                match result {
                    Ok(recents) => self.recents = recents,
                    Err(error) => tracing::warn!("icon recents unavailable: {error}"),
                }
                Task::none()
            }
            Message::BundledLoaded(result) => {
                match result {
                    Ok(bundled) => self.java.bundled = bundled,
                    Err(error) => tracing::warn!("bundled Java runtimes unavailable: {error}"),
                }
                Task::none()
            }
            Message::SystemJavaLoaded(major) => {
                self.java.system = major;
                Task::none()
            }
            Message::MemoryLoaded(gb) => {
                self.ceiling_gb = gb;
                Task::none()
            }
            Message::VersionsLoaded(result) => {
                match result {
                    Ok(versions) => {
                        self.versions = versions;
                        self.versions_error = None;
                    }
                    Err(error) => self.versions_error = Some(error),
                }
                Task::none()
            }
            Message::LoadBuilds => {
                // The builds shown belong to the previous version, so they wait for the new list.
                self.builds = None;
                self.builds_error = None;
                if instance.loader == Loader::Vanilla {
                    return Task::none();
                }
                builds_of(instance)
            }
            Message::BuildsLoaded { mc, result } => {
                if mc != instance.mc_version {
                    return Task::none();
                }
                match result {
                    Ok(builds) => {
                        // A new version has no loader build yet, so the newest stable one is picked, as New Instance does.
                        let pick = instance
                            .loader_version
                            .is_none()
                            .then(|| {
                                builds
                                    .iter()
                                    .find(|build| build.stable)
                                    .or_else(|| builds.first())
                                    .map(|build| build.version.clone())
                            })
                            .flatten();
                        self.builds = Some(builds);
                        self.builds_error = None;
                        return pick.map_or_else(Task::none, |version| edit_task(Edit::LoaderBuild(version)));
                    }
                    Err(error) => {
                        self.builds = Some(Vec::new());
                        self.builds_error = Some(error);
                    }
                }
                Task::none()
            }
            // The root applies these. The page never sees them, since the root answers before it.
            Message::Edit(_) | Message::ResetSettings | Message::Reinstall | Message::RevealFolder | Message::DeleteInstance => Task::none(),
        }
    }

    /// Takes a typed digit for one side of the resolution. Once both sides are a valid pair, the pair is saved.
    fn resolution(&mut self, axis: Axis, text: &str) -> Task<Message> {
        let text = launch_rows::sanitize_dimension(text);
        match axis {
            Axis::Width => self.draft.width = Some(text),
            Axis::Height => self.draft.height = Some(text),
        }
        let Some(saved) = self.resolved.as_ref().map(|resolved| resolved.resolution) else {
            return Task::none();
        };
        let (width, height) = self.draft.texts(saved);
        match (launch_rows::parse_dimension(&width), launch_rows::parse_dimension(&height)) {
            (Some(width), Some(height)) => {
                self.draft = ResolutionDraft::default();
                let resolution = Resolution { width, height };
                if resolution == saved {
                    Task::none()
                } else {
                    edit_task(Edit::Launch(LaunchChange::Resolution(resolution)))
                }
            }
            _ => Task::none(),
        }
    }

    /// Opens the banner editor on the instance's scene, then loads its uploaded picture when it has one.
    fn open_banner(&mut self, instance: &Instance) -> Task<Message> {
        let index = scene_index(instance.biome, instance.art_seed);
        self.banner = Some(banner_editor::BannerEditor::new(&BannerChoice::Scene(index)));
        match &instance.art_path {
            Some(path) => Task::perform(read_picture(path.clone()), Message::BannerCurrent),
            None => Task::none(),
        }
    }

    fn set_banner_picture(&mut self, result: Result<Vec<u8>, String>) {
        match result {
            Ok(png) => {
                if let Some(editor) = &mut self.banner {
                    editor.set_upload(Upload::new(png));
                }
            }
            Err(error) => tracing::warn!("reading the banner failed: {error}"),
        }
    }

    fn banner_message(&mut self, message: banner_editor::Message) -> Task<Message> {
        let Some(editor) = &mut self.banner else {
            return Task::none();
        };
        match editor.update(message) {
            banner_editor::Outcome::Stay => Task::none(),
            banner_editor::Outcome::Upload => Task::perform(upload::pick_banner(), Message::BannerPicked),
            banner_editor::Outcome::Save(choice) => {
                self.banner = None;
                edit_task(Edit::Banner(choice))
            }
            banner_editor::Outcome::Cancel => {
                self.banner = None;
                Task::none()
            }
        }
    }

    /// Opens the icon editor on the instance's icon. An uploaded icon is read so the editor can show it.
    fn open_icon(&mut self, instance: &Instance) -> Task<Message> {
        let choice = match &instance.icon {
            InstanceIcon::Composed { background, symbol } => IconChoice::Composed {
                background: instance_icon::preset(background).id,
                symbol: instance_icon::symbol(symbol).id,
            },
            InstanceIcon::Custom(_) => {
                let (background, symbol) = instance_icon::random();
                IconChoice::Composed { background, symbol }
            }
        };
        self.icon = Some(icon_editor::IconEditor::new(&choice, self.recents.clone()));
        match &instance.icon {
            InstanceIcon::Custom(path) => Task::perform(read_picture(path.clone()), Message::IconCurrent),
            InstanceIcon::Composed { .. } => Task::none(),
        }
    }

    fn set_icon_picture(&mut self, result: Result<Vec<u8>, String>) {
        match result {
            Ok(png) => {
                if let Some(editor) = &mut self.icon {
                    editor.set_upload(Upload::new(png));
                }
            }
            Err(error) => tracing::warn!("reading the icon failed: {error}"),
        }
    }

    fn icon_message(&mut self, message: icon_editor::Message) -> Task<Message> {
        let Some(editor) = &mut self.icon else {
            return Task::none();
        };
        // A new symbol may sit in a row the grid is not showing, so bring its row into view.
        let reveal = matches!(message, icon_editor::Message::Randomize | icon_editor::Message::RecentPicked(_));
        match editor.update(message) {
            icon_editor::Outcome::Stay if reveal => editor.reveal_symbol(),
            icon_editor::Outcome::Stay => Task::none(),
            icon_editor::Outcome::Upload => Task::perform(upload::pick_icon(), Message::IconPicked),
            icon_editor::Outcome::Save(choice) => {
                self.icon = None;
                edit_task(Edit::Icon(choice))
            }
            icon_editor::Outcome::Cancel => {
                self.icon = None;
                Task::none()
            }
        }
    }

    /// The tab: the sidebar, then the page of sections, which scrolls on its own beside the sidebar.
    pub fn view<'a>(&'a self, instance: &'a Instance, folder: &'a Path) -> Element<'a, Message> {
        let Some(resolved) = self.resolved.as_ref() else {
            return Space::new().into();
        };
        let shown = Shown {
            values: resolved,
            overrides: self.overrides.as_ref(),
            java: &self.java,
            ceiling_gb: self.ceiling_gb,
        };
        let inherited = self.overrides.as_ref().map_or(0, InstanceSettings::inherited_count);
        let page = column![
            block(Section::General, general::body(instance)),
            block(Section::JavaMemory, launch_rows::java_memory(shown, launch_edit)),
            block(Section::Window, launch_rows::window(shown, &self.draft, launch_edit, Message::Resolution),),
            block(Section::LaunchArgs, launch_rows::launch_args(shown, launch_edit)),
            block(
                Section::GameVersion,
                sections::game_version(
                    instance,
                    &self.versions,
                    self.versions_error.as_deref(),
                    self.builds.as_deref(),
                    self.builds_error.as_deref(),
                ),
            ),
            block(Section::PathsFiles, sections::paths_files(folder)),
            block(Section::Advanced, launch_rows::advanced(shown, launch_edit, Message::OpenEnvironment)),
            block(Section::DangerZone, sections::danger_zone()),
        ]
        .spacing(40)
        .width(Fill);
        let content = container(page).id(nav::column_id()).width(Fill).padding(Padding::new(0.0).bottom(40.0));

        row![
            nav::sidebar(self.section, inherited),
            scrollbar::scroll(content)
                .id(nav::content_id())
                .on_scroll(nav::on_scroll)
                .width(Fill)
                .height(Fill),
        ]
        .spacing(28)
        .height(Fill)
        .align_y(alignment::Vertical::Top)
        .into()
    }

    /// The editors and the environment modal, over the page, while one is open.
    pub fn overlay(&self) -> Option<Element<'_, Message>> {
        if self.environment_open {
            let vars: &[EnvVar] = self.resolved.as_ref().map_or(&[], |resolved| resolved.env.as_slice());
            return Some(launch_rows::environment_modal(vars, Message::Environment, Message::CloseEnvironment));
        }
        if let Some(editor) = &self.banner {
            return Some(banner_editor::modal(editor).map(Message::BannerEditor));
        }
        self.icon.as_ref().map(|editor| icon_editor::modal(editor).map(Message::IconEditor))
    }
}

/// A launch change, as the rows emit it.
fn launch_edit(change: LaunchChange) -> Message {
    Message::Edit(Edit::Launch(change))
}

/// A section's container: its heading, then its body. The container carries the anchor the page scrolls to.
fn block<'a>(section: Section, body: Element<'a, Message>) -> Element<'a, Message> {
    container(
        column![section_header(section.icon(), section.tint(), section.label(), section.subtitle()), body,]
            .spacing(16)
            .width(Fill),
    )
    .id(section.anchor_id())
    .width(Fill)
    .into()
}

/// The task that hands `edit` to the root.
fn edit_task(edit: Edit) -> Task<Message> {
    Task::done(Message::Edit(edit))
}

/// The task that reports a picture error to the root.
fn failed(error: String) -> Task<Message> {
    edit_task(Edit::Failed(error))
}

/// The task that loads the loader builds of the instance's Minecraft version.
fn builds_of(instance: &Instance) -> Task<Message> {
    let loader = instance.loader;
    let mc = instance.mc_version.clone();
    let query = mc.clone();
    Task::perform(async move { catalog::loader_builds(loader, &query).await }, move |result| {
        Message::BuildsLoaded { mc: mc.clone(), result }
    })
}

/// The index of the banner scene an instance's biome and art seed show, or the first scene when none matches.
pub fn scene_index(biome: Biome, seed: u64) -> usize {
    (0..SCENE_COUNT).find(|index| banner::scene(*index) == (biome, seed)).unwrap_or(0)
}

/// Reads a picture file's bytes on the blocking pool.
async fn read_picture(path: PathBuf) -> Result<Vec<u8>, String> {
    tokio::task::spawn_blocking(move || std::fs::read(&path).map_err(|error| error.to_string()))
        .await
        .map_err(|error| error.to_string())?
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/settings/mod.rs"]
mod tests;
