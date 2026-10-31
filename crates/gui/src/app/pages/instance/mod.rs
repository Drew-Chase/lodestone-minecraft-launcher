//! The instance detail page — the immersive per-instance view from
//! `04-instance.png`.
//!
//! The page is a fixed masthead plus tab row over one content region. Each tab
//! lives in its own module with its own state, message and view; this file
//! routes messages and lays out the shell. The instance model's own data (name,
//! loader, version, playtime, biome) comes from the database. Everything else
//! the tabs show is read from the instance folder by [`files`], on the blocking
//! pool, and applied when it lands.

pub mod files;
pub(crate) mod format;
mod hero;
pub(crate) mod logs;
mod mods;
mod overview;
mod screenshots;
pub mod settings;
mod worlds;

use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Duration;

use chrono::Local;
use iced::widget::image::Handle;
use iced::widget::{Text, column, container, stack};
use iced::{Element, Fill, Padding, Size, Subscription, Task, alignment};

use crate::app::components::Tabs;
use crate::app::components::instance_card::install_status;
use crate::app::components::scrollbar;
use crate::app::downloads::Pending;
use crate::app::lang;
use crate::app::launch::games::GameView;
use crate::app::pages::Page;
use crate::app::pages::library::instance::Instance;
use crate::app::pages::library::instance_menu;
use crate::app::theme::{color, fonts};
use files::InstanceFiles;

/// The page's horizontal gutter — 28px, per the design.
const GUTTER: f32 = 28.0;

/// The page's tabs, in tab-bar order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    Overview,
    Mods,
    Worlds,
    Screenshots,
    Logs,
    Settings,
}

impl Tab {
    pub const ALL: [Tab; 6] = [Tab::Overview, Tab::Mods, Tab::Worlds, Tab::Screenshots, Tab::Logs, Tab::Settings];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Overview => lang::t("instance.tab.overview"),
            Tab::Mods => lang::t("instance.tab.mods"),
            Tab::Worlds => lang::t("instance.tab.worlds"),
            Tab::Screenshots => lang::t("instance.tab.screenshots"),
            Tab::Logs => lang::t("instance.tab.logs"),
            Tab::Settings => lang::t("instance.tab.settings"),
        }
    }
}

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
    /// Requests navigation — the root intercepts this before the page's
    /// `update` runs (the `<Link>` of this framework).
    Navigate(Page),
    Mods(mods::Message),
    Worlds(worlds::Message),
    Logs(logs::Message),
    Settings(settings::Message),
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
    worlds: worlds::State,
    screenshots: screenshots::State,
    logs: logs::State,
    settings: settings::State,
    /// The run whose console the Logs tab shows, when the runs panel opened one. `None` shows the
    /// newest run.
    console_run: Option<u64>,
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

    /// Points the page at `instance`, whose files live in `folder`. Resets the
    /// transient controls and every tab, and returns the task that reads the folder.
    pub fn show(&mut self, instance: Instance, folder: PathBuf) -> Task<Message> {
        self.generation += 1;
        let generation = self.generation;
        self.settings = settings::State::for_instance(&instance);
        self.instance = Some(instance);
        self.folder = folder.clone();
        self.files = None;
        self.mods = mods::State::default();
        self.worlds = worlds::State::default();
        self.screenshots = screenshots::State::default();
        self.logs = logs::State::default();
        self.tab = Tab::Overview;
        self.console_run = None;

        Task::perform(
            async move { tokio::task::spawn_blocking(move || files::scan(&folder)).await.unwrap_or_default() },
            move |files| Message::Scanned { generation, files },
        )
    }

    /// Replaces the shown instance's copy with a reload of it, so its state and playtime are
    /// current. A reload of any other instance is ignored.
    pub fn refresh(&mut self, instance: Instance) {
        if self.instance.as_ref().is_some_and(|shown| shown.id == instance.id) {
            self.instance = Some(instance);
        }
    }

    /// The instance the page shows, if one has been shown.
    pub fn instance(&self) -> Option<&Instance> {
        self.instance.as_ref()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TabSelected(tab) => {
                self.tab = tab;
                Task::none()
            }
            Message::Mods(message) => {
                self.mods.update(message);
                Task::none()
            }
            Message::Worlds(message) => {
                self.worlds.update(message);
                Task::none()
            }
            Message::Logs(message) => {
                self.logs.update(message);
                Task::none()
            }
            Message::Settings(message) => {
                self.settings.update(message);
                Task::none()
            }
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
            | Message::LaunchNew(_) => Task::none(),
            // Results for an instance the page no longer shows, and ticks the
            // Logs tab no longer needs.
            Message::Scanned { .. } | Message::Thumbnail { .. } | Message::LogsTick | Message::LogsLoaded { .. } => Task::none(),
        }
    }

    /// The log refresh: while the Logs tab is following, the log is re-read every
    /// two seconds. The root only adds this while the detail page is on screen.
    pub fn subscription(&self) -> Subscription<Message> {
        if self.tab == Tab::Logs && self.logs.following() {
            iced::time::every(Duration::from_secs(2)).map(|_| Message::LogsTick)
        } else {
            Subscription::none()
        }
    }

    /// Hands a scan's results to the tabs and starts the thumbnails.
    fn apply(&mut self, files: InstanceFiles) -> Task<Message> {
        self.mods.set_mods(files.mods.clone());
        self.worlds.set_worlds(files.worlds.clone(), Local::now());
        self.screenshots.set_shots(files.screenshots.clone());
        self.logs.set_lines(files.log.clone());
        self.files = Some(files);
        self.decode_next_thumbnail()
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

    /// The page: fixed masthead, fixed tab row, and the tab body.
    pub fn view<'a>(&'a self, viewport: Size, game: GameView<'a>, install: Option<&'a Pending>) -> Element<'a, Message> {
        let Some(instance) = &self.instance else {
            return self.empty_state();
        };

        let controls = hero::Controls {
            status: game.status,
            install: install.map(install_status),
        };
        let page = column![
            hero::hero(instance, self.files.as_ref(), controls, viewport),
            self.tab_bar(),
            self.tab_body(instance, game.console, install),
        ]
        .width(Fill)
        .height(Fill);

        // The icon hangs 40px below the masthead, so it is layered over the
        // whole page: inside the masthead it would be clamped to the 70px
        // left beneath its top offset.
        stack![page, hero::hero_icon(instance)].width(Fill).height(Fill).into()
    }

    fn empty_state(&self) -> Element<'_, Message> {
        container(
            column![
                Text::new(lang::t("instance.none_selected"))
                    .size(18)
                    .font(fonts::Inter::EXTRA_BOLD)
                    .color(color::Text::HEADLINE),
                Text::new(lang::t("instance.none_selected_hint"))
                    .size(12)
                    .font(fonts::Inter::REGULAR)
                    .color(color::Text::MUTED),
                crate::app::components::TextButton::ghost(lang::t("instance.back_to_library")).on_press(Message::Back),
            ]
            .spacing(12)
            .align_x(alignment::Horizontal::Center),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .into()
    }

    /// The underlined tab row. `Tabs` draws the full-width rule and the active
    /// bar itself, so the bar paints over the rule and every tab's whole slot
    /// is clickable.
    fn tab_bar(&self) -> Element<'_, Message> {
        let active = Tab::ALL.iter().position(|tab| *tab == self.tab).unwrap_or_default();

        container(
            Tabs::new(Tab::ALL.map(Tab::label))
                .underline()
                .inset(GUTTER)
                .width(Fill)
                .active(active)
                .on_select(|index| Message::TabSelected(Tab::ALL[index])),
        )
        .width(Fill)
        .padding(Padding::new(0.0).top(52.0))
        .into()
    }

    /// The selected tab's content, padded to the page gutter. The log panel
    /// fills the height and scrolls itself; every other tab scrolls as a page.
    fn tab_body<'a>(
        &'a self,
        instance: &'a Instance,
        console: Option<&'a VecDeque<logs::LogLine>>,
        install: Option<&'a Pending>,
    ) -> Element<'a, Message> {
        let content: Element<'a, Message> = match self.tab {
            Tab::Overview => overview::view(instance, &self.screenshots, install),
            Tab::Mods => self.mods.view().map(Message::Mods),
            Tab::Worlds => self.worlds.view().map(Message::Worlds),
            Tab::Screenshots => screenshots::view(&self.screenshots),
            Tab::Logs => self.logs.view(console).map(Message::Logs),
            Tab::Settings => self.settings.view(instance).map(Message::Settings),
        };
        let padded = container(content)
            .width(Fill)
            .padding(Padding::new(0.0).left(GUTTER).right(GUTTER).top(22.0).bottom(40.0));

        match self.tab {
            Tab::Logs => padded.height(Fill).into(),
            _ => scrollbar::scroll(padded).width(Fill).height(Fill).into(),
        }
    }
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
