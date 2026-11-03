//! Settings: the launcher's preferences, in six sections on one scrolling page.
//!
//! The sections edit a [`Draft`] of [`SettingsValues`]. The sidebar lights the section at the top of the page, and presses
//! scroll to a section. Save changes commits the draft, and the root applies and saves the values. Reset throws the edits
//! away. The launch defaults in the draft are the ones every instance inherits until it overrides one.

pub mod accounts;
mod appearance;
mod draft;
mod edit;
mod general;
mod java_memory;
mod nav;
pub mod network;
mod privacy;

use std::path::{MAIN_SEPARATOR, Path, PathBuf};

use aurora_leaf::Icon;
use iced::widget::{Id, Space, column, container, row, stack, text, text_editor};
use iced::{Element, Fill, Padding, Task, alignment};

use self::draft::Draft;
pub use self::edit::Edit;
use self::network::speed::Meter;
use crate::app::components::button::Variant;
use crate::app::components::launch_rows::{self, Axis, EnvEdit, JavaOptions, ResolutionDraft, Shown};
use crate::app::components::panel::{hairline, panel_with_padding};
use crate::app::components::scroll_spy;
use crate::app::components::setting_row::section_header;
use crate::app::components::{TextButton, scrollbar};
use crate::app::install::catalog::{self, BundledJava};
use crate::app::install::java;
use crate::app::lang;
use crate::app::launch_settings::{InstanceSettings, JavaChoice, LaunchChange, LaunchSettings, Resolution};
use crate::app::pages::Page;
use crate::app::paths::LauncherPaths;
use crate::app::preferences::{SettingsValues, StartupPage};
use crate::app::storage::StoredCredentials;
use crate::app::theme::{Accent, color, fonts, parse_hex};
use iced::widget::image;

/// The Settings page: the draft the tabs edit, and what the rows need to show it.
#[derive(Debug)]
pub struct Settings {
    /// The saved values and the copy the tabs edit.
    draft: Draft,
    /// The launch settings the draft's defaults resolve to, which the launch rows show.
    values: LaunchSettings,
    /// The launcher's folders. The Java tab looks for the runtimes installed in them.
    paths: LauncherPaths,
    /// The folder every instance keeps its files in, which the General tab shows.
    instances_dir: PathBuf,
    /// The section at the top of the page, which the sidebar lights.
    tab: Tab,
    /// Where each section's top sits, measured on the last scroll or press.
    offsets: Vec<(Tab, f32)>,
    /// The page's scroll offset, and whether it is at its bottom with more above.
    scroll: f32,
    at_end: bool,
    /// The custom accent's hex as it is typed. It becomes the accent once it is a valid color.
    custom_hex: String,
    java: JavaOptions,
    /// The bundled runtimes installed on this machine, which the Installed runtimes list shows.
    runtimes: Vec<Runtime>,
    /// The JVM arguments as the editor holds them while they are typed.
    jvm: text_editor::Content,
    /// The largest heap the slider offers, in gigabytes. Set from the machine's memory once the page opens.
    ceiling_gb: u32,
    /// The width and height fields while they are being typed.
    resolution: ResolutionDraft,
    environment_open: bool,
}

/// Messages produced by the settings page.
#[derive(Debug, Clone)]
pub enum Message {
    /// Opens another page. The root routes it.
    Navigate(Page),
    /// A sidebar entry was pressed: the page scrolls to that section.
    Section(Tab),
    /// The page scrolled. `at_end` is set when it is at its bottom with more above.
    Scrolled {
        offset: f32,
        at_end: bool,
    },
    /// Where each section's top sits on the page.
    Measured(Vec<(Tab, f32)>),
    /// A launch default changed.
    Change(LaunchChange),
    /// A setting other than a launch default changed.
    Edit(Edit),
    /// The width or height field was typed in.
    Resolution(Axis, String),
    /// An edit or a movement in the JVM arguments editor.
    JvmEdit(text_editor::Action),
    /// The custom accent's hex was typed in.
    CustomHex(String),
    OpenEnvironment,
    CloseEnvironment,
    /// One change to the environment variables.
    Environment(EnvEdit),
    /// The runtimes the platform bundles.
    BundledLoaded(Result<Vec<BundledJava>, String>),
    /// The major version of the system Java, when one was found.
    SystemJavaLoaded(Option<u32>),
    /// The machine's memory, in whole gigabytes.
    MemoryLoaded(u32),
    /// An account was switched, signed out of, or added. The root carries it out.
    Account(accounts::AccountAction),
    /// Opens the folder instances are kept in. The root reveals it.
    OpenInstancesFolder,
    /// Opens the folder the launcher keeps its data in. The root reveals it.
    OpenDataFolder,
    /// Saves the saved settings to a file the user picks.
    ExportSettings,
    /// Asks before the saved settings and the sign-ins are cleared. The root asks.
    ClearLocalData,
    /// Commits the draft. The root applies the values and saves them.
    Save,
    /// Throws away the edits.
    Reset,
}

/// The tabs, in the order the sidebar lists them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    General,
    Accounts,
    JavaMemory,
    Appearance,
    Network,
    Privacy,
}

impl Tab {
    /// Every tab, in sidebar order.
    pub const ALL: [Tab; 6] = [Tab::General, Tab::Accounts, Tab::JavaMemory, Tab::Appearance, Tab::Network, Tab::Privacy];

    /// The tab's name: its sidebar label and its heading.
    pub(super) fn title_key(self) -> &'static str {
        match self {
            Tab::General => "settings.general.title",
            Tab::Accounts => "settings.accounts.title",
            Tab::JavaMemory => "settings.java_memory.title",
            Tab::Appearance => "settings.appearance.title",
            Tab::Network => "settings.network.title",
            Tab::Privacy => "settings.privacy.title",
        }
    }

    /// The line under the tab's heading.
    fn subtitle_key(self) -> &'static str {
        match self {
            Tab::General => "settings.general.subtitle",
            Tab::Accounts => "settings.accounts.subtitle",
            Tab::JavaMemory => "settings.java_memory.subtitle",
            Tab::Appearance => "settings.appearance.subtitle",
            Tab::Network => "settings.network.subtitle",
            Tab::Privacy => "settings.privacy.subtitle",
        }
    }

    /// The icon in the sidebar.
    pub(super) fn icon(self) -> &'static str {
        match self {
            Tab::General => Icon::lucide().settings(),
            Tab::Accounts => Icon::lucide().user(),
            Tab::JavaMemory => Icon::lucide().cpu(),
            Tab::Appearance => Icon::lucide().image(),
            Tab::Network => Icon::lucide().globe(),
            Tab::Privacy => Icon::lucide().lock(),
        }
    }

    /// The id of the section's container, which the page measures and scrolls to.
    pub fn anchor_id(self) -> Id {
        Id::new(match self {
            Tab::General => "settings-general",
            Tab::Accounts => "settings-accounts",
            Tab::JavaMemory => "settings-java-memory",
            Tab::Appearance => "settings-appearance",
            Tab::Network => "settings-network",
            Tab::Privacy => "settings-privacy",
        })
    }
}

/// A bundled runtime that is installed on this machine, as the Installed runtimes list shows it.
#[derive(Debug, Clone)]
pub struct Runtime {
    /// The launch choice that selects this runtime.
    pub choice: JavaChoice,
    /// The runtime's name, such as `Java 21 · Temurin`.
    pub label: String,
    /// Where the runtime is installed, with the home folder as `~`.
    pub path: String,
}

impl Default for Settings {
    fn default() -> Self {
        let draft = Draft::new(SettingsValues::default());
        let values = InstanceSettings::default().resolve(&draft.current().instance_defaults);
        let jvm = text_editor::Content::with_text(&draft.current().instance_defaults.jvm_args);
        Self {
            draft,
            values,
            paths: LauncherPaths::default(),
            instances_dir: PathBuf::new(),
            tab: Tab::default(),
            offsets: Vec::new(),
            scroll: 0.0,
            at_end: false,
            custom_hex: "#47d9ff".to_owned(),
            java: JavaOptions::default(),
            runtimes: Vec::new(),
            jvm,
            ceiling_gb: 0,
            resolution: ResolutionDraft::default(),
            environment_open: false,
        }
    }
}

impl Settings {
    /// Takes the saved values as the draft's saved copy, and the launcher's folders. Called whenever the page opens. The page
    /// starts at its top, with the first section lit.
    pub fn sync(&mut self, saved: &SettingsValues, paths: &LauncherPaths) {
        self.draft.reload(saved);
        self.paths = paths.clone();
        self.instances_dir = paths.instances();
        self.tab = Tab::default();
        self.offsets.clear();
        self.scroll = 0.0;
        self.at_end = false;
        self.refresh();
    }

    /// Commits the draft, and returns the values for the root to apply and save.
    pub fn commit(&mut self) -> SettingsValues {
        self.draft.commit()
    }

    /// Takes `saved` as the saved values and drops the edits in progress. The root calls it once the saved settings are cleared.
    pub fn replace_saved(&mut self, saved: &SettingsValues, paths: &LauncherPaths) {
        self.draft = Draft::new(saved.clone());
        self.sync(saved, paths);
    }

    /// Loads the lists the rows offer: the bundled runtimes, the system Java and the machine's memory. Called when the
    /// page opens.
    pub fn open(&self) -> Task<Message> {
        Task::batch([
            Task::perform(catalog::bundled_java(), Message::BundledLoaded),
            Task::perform(java::system(1), |found| Message::SystemJavaLoaded(found.map(|java| java.major))),
            Task::perform(
                async { tokio::task::spawn_blocking(launch_rows::machine_memory_gb).await.unwrap_or(16) },
                Message::MemoryLoaded,
            ),
        ])
    }

    /// Applies a message. Navigation, opening the folder and saving are the root's, so they leave here untouched.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Navigate(_)
            | Message::OpenInstancesFolder
            | Message::OpenDataFolder
            | Message::ClearLocalData
            | Message::Save
            | Message::Account(_) => Task::none(),
            Message::ExportSettings => self.export_settings(),
            Message::Section(tab) => {
                self.tab = tab;
                nav::scroll_to(tab)
            }
            Message::Scrolled { offset, at_end } => {
                self.scroll = offset;
                self.at_end = at_end;
                nav::measure()
            }
            Message::Measured(offsets) => {
                self.offsets = offsets;
                if let Some(tab) = scroll_spy::active(&self.offsets, self.scroll, self.at_end) {
                    self.tab = tab;
                }
                Task::none()
            }
            Message::Change(change) => {
                self.draft.current_mut().instance_defaults.apply(change);
                self.refresh();
                Task::none()
            }
            Message::CustomHex(text) => {
                if parse_hex(&text).is_some() {
                    Edit::Accent(Accent::Custom { hex: text.clone() }).apply(self.draft.current_mut());
                    self.refresh();
                }
                self.custom_hex = text;
                Task::none()
            }
            Message::Edit(edit) => {
                edit.apply(self.draft.current_mut());
                Task::none()
            }
            Message::Reset => {
                self.draft.reset();
                self.refresh();
                Task::none()
            }
            Message::Resolution(axis, text) => self.resolution(axis, &text),
            Message::JvmEdit(action) => {
                let editing = action.is_edit();
                self.jvm.perform(action);
                if editing {
                    self.draft.current_mut().instance_defaults.jvm_args = self.jvm.text();
                    self.refresh();
                }
                Task::none()
            }
            Message::OpenEnvironment => {
                self.environment_open = true;
                Task::none()
            }
            Message::CloseEnvironment => {
                self.environment_open = false;
                Task::none()
            }
            Message::Environment(edit) => {
                edit.apply(&mut self.draft.current_mut().instance_defaults.env);
                self.refresh();
                Task::none()
            }
            Message::BundledLoaded(result) => {
                match result {
                    Ok(bundled) => {
                        self.runtimes = installed_runtimes(&self.paths, &bundled);
                        self.java.bundled = bundled;
                    }
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
        }
    }

    /// Asks where to save the saved values, then writes them there as JSON. A failure is logged, not shown.
    fn export_settings(&self) -> Task<Message> {
        let Ok(json) = serde_json::to_vec_pretty(self.draft.saved()) else {
            return Task::none();
        };
        Task::future(async move {
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_title(lang::t("settings.privacy.export_title"))
                .set_file_name("lodestone-settings.json")
                .add_filter(lang::t("settings.privacy.export_filter"), &["json"])
                .save_file()
                .await
            else {
                return;
            };
            if let Err(error) = file.write(&json).await {
                tracing::warn!("Exporting the settings failed: {error}");
            }
        })
        .discard()
    }

    /// Resolves the draft's launch defaults into the values the rows show, and keeps the JVM editor on the draft's text.
    fn refresh(&mut self) {
        let defaults = &self.draft.current().instance_defaults;
        self.values = InstanceSettings::default().resolve(defaults);
        if self.jvm.text() != defaults.jvm_args {
            self.jvm = text_editor::Content::with_text(&defaults.jvm_args);
        }
    }

    /// Takes a typed digit for one side of the resolution, and saves the pair once both sides are valid.
    fn resolution(&mut self, axis: Axis, text: &str) -> Task<Message> {
        let text = launch_rows::sanitize_dimension(text);
        match axis {
            Axis::Width => self.resolution.width = Some(text),
            Axis::Height => self.resolution.height = Some(text),
        }
        let saved = self.draft.current().instance_defaults.resolution;
        let (width, height) = self.resolution.texts(saved);
        if let (Some(width), Some(height)) = (launch_rows::parse_dimension(&width), launch_rows::parse_dimension(&height)) {
            self.resolution = ResolutionDraft::default();
            self.draft.current_mut().instance_defaults.resolution = Resolution { width, height };
            self.refresh();
        }
        Task::none()
    }

    /// The page: the sidebar of sections, then the heading and every section in one scrolling column. The environment editor
    /// covers it while open.
    pub fn view<'a>(
        &'a self,
        connected: Option<bool>,
        meter: &'a Meter,
        accounts: &'a StoredCredentials,
        face: Option<&'a image::Handle>,
    ) -> Element<'a, Message> {
        let shown = Shown {
            values: &self.values,
            overrides: None,
            java: &self.java,
            ceiling_gb: self.ceiling_gb,
        };
        let sections = column![
            block(
                Tab::General,
                general::view(shown, &self.resolution, self.draft.current(), &self.instances_dir),
            ),
            block(Tab::Accounts, accounts::view(accounts, face)),
            block(Tab::JavaMemory, java_memory::view(shown, &self.jvm, &self.runtimes)),
            block(Tab::Appearance, appearance::view(self.draft.current(), &self.custom_hex)),
            block(Tab::Network, network::view(self.draft.current(), meter, connected)),
            block(Tab::Privacy, privacy::view(self.draft.current())),
        ]
        .spacing(40)
        .width(Fill);
        let content = container(sections)
            .id(nav::column_id())
            .width(Fill)
            .padding(Padding::new(0.0).left(28.0).right(28.0).top(8.0).bottom(40.0));
        let main = column![
            header(self.draft.dirty()),
            hairline(),
            scrollbar::scroll(content)
                .id(nav::content_id())
                .on_scroll(nav::on_scroll)
                .width(Fill)
                .height(Fill),
        ]
        .width(Fill)
        .height(Fill);
        let layout = row![nav::view(self.tab), main].width(Fill).height(Fill);

        if self.environment_open {
            let modal = launch_rows::environment_modal(
                &self.draft.current().instance_defaults.env,
                Message::Environment,
                Message::CloseEnvironment,
            );
            return stack![layout, modal].width(Fill).height(Fill).into();
        }
        layout.into()
    }
}

/// The runtimes of `bundled` that finished installing under the launcher's folders, in the order the platform lists them.
fn installed_runtimes(paths: &LauncherPaths, bundled: &[BundledJava]) -> Vec<Runtime> {
    bundled
        .iter()
        .filter_map(|java_runtime| {
            let home = java::installed_home(paths, &java_runtime.component)?;
            Some(Runtime {
                choice: JavaChoice::Bundled {
                    component: java_runtime.component.clone(),
                },
                label: java_runtime.label.clone(),
                path: display_path(&home),
            })
        })
        .collect()
}

/// The path with the home folder written as `~`, as the rest of the page shows it.
pub(super) fn display_path(path: &Path) -> String {
    match dirs::home_dir().and_then(|home| path.strip_prefix(&home).ok().map(Path::to_path_buf)) {
        Some(rest) => format!("~{MAIN_SEPARATOR}{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// The heading: the page's title and subtitle, then Reset and Save changes, which act on the draft.
fn header<'a>(dirty: bool) -> Element<'a, Message> {
    let reset = TextButton::labeled_icon(Icon::lucide().rotate_ccw(), lang::t("settings.reset"), Variant::Ghost)
        .disabled(!dirty)
        .on_press(Message::Reset);
    let save = TextButton::labeled_icon(Icon::lucide().save(), lang::t("settings.save_changes"), Variant::Primary)
        .disabled(!dirty)
        .on_press(Message::Save);
    row![
        column![
            crate::h2!(lang::t("settings.page.title")),
            crate::body!(lang::t("settings.page.subtitle")),
        ]
        .spacing(8),
        Space::new().width(Fill),
        reset,
        save,
    ]
    .spacing(12)
    .align_y(alignment::Vertical::Center)
    .padding(Padding::new(0.0).left(28.0).right(28.0).top(28.0).bottom(24.0))
    .into()
}

/// A section of the page: its heading, then its body. The container carries the anchor the sidebar scrolls to.
fn block<'a>(tab: Tab, body: Element<'a, Message>) -> Element<'a, Message> {
    container(
        column![
            section_header(tab.icon(), color::primary(), lang::t(tab.title_key()), lang::t(tab.subtitle_key())),
            body,
        ]
        .spacing(16)
        .width(Fill),
    )
    .id(tab.anchor_id())
    .width(Fill)
    .into()
}

/// A card with a title and a subtitle over its body.
pub(super) fn card<'a>(title: &'static str, subtitle: &'static str, body: Element<'a, Message>) -> Element<'a, Message> {
    let heading = column![
        text(title).size(16).font(fonts::Inter::SEMI_BOLD).color(color::text_headline()),
        text(subtitle).size(13).font(fonts::Inter::REGULAR).color(color::text_muted()),
    ]
    .spacing(4);
    container(panel_with_padding(
        column![heading, body].spacing(18),
        Padding::new(0.0).left(24.0).right(24.0).top(22.0).bottom(22.0),
    ))
    .width(Fill)
    .into()
}

/// A card with no heading: only its body, such as a list of rows.
pub(super) fn plain_card<'a>(body: Element<'a, Message>) -> Element<'a, Message> {
    container(panel_with_padding(body, Padding::new(0.0).left(24.0).right(24.0)))
        .width(Fill)
        .into()
}

/// The page the launcher opens on for a startup choice.
pub fn startup_page(page: StartupPage) -> Page {
    match page {
        StartupPage::Library => Page::Library,
        StartupPage::News => Page::News,
        StartupPage::Discover => Page::Discover,
        StartupPage::Worlds => Page::Worlds,
    }
}
