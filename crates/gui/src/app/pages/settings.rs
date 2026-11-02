//! Settings: the launcher's preferences. Its instance defaults are the launch settings every instance inherits until it
//! overrides one. The root keeps and saves them, and the page edits them through the same rows an instance's Settings
//! tab uses.

use aurora_leaf::Icon;
use iced::widget::{Space, column, container, row, stack};
use iced::{Element, Fill, Padding, Task};

use crate::app::components::launch_rows::{self, Axis, EnvEdit, JavaOptions, ResolutionDraft, Shown};
use crate::app::components::setting_row::section_header;
use crate::app::components::{TextButton, scrollbar};
use crate::app::install::catalog::{self, BundledJava};
use crate::app::install::java;
use crate::app::lang;
use crate::app::launch_settings::{InstanceSettings, LaunchChange, LaunchSettings};
use crate::app::pages::Page;
use crate::app::theme::color;

/// State of the settings page: the defaults as the rows show them, the lists the rows offer, and what is open.
#[derive(Debug, Default)]
pub struct Settings {
    /// The launch defaults, copied from the root whenever they change.
    values: Option<LaunchSettings>,
    java: JavaOptions,
    /// The largest heap the slider offers, in gigabytes. Set from the machine's memory once the page opens.
    ceiling_gb: u32,
    draft: ResolutionDraft,
    environment_open: bool,
}

/// Messages produced by the settings page.
#[derive(Debug, Clone)]
pub enum Message {
    /// Requests navigation — the root model intercepts this before it reaches [`Settings::update`] (see the contract in
    /// `pages/mod.rs`).
    Navigate(Page),
    /// A default changed. The root applies it and saves it.
    Change(LaunchChange),
    /// The width or height field was typed in.
    Resolution(Axis, String),
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
}

impl Settings {
    /// Shows the launch defaults. Called whenever the root's defaults change.
    pub fn sync(&mut self, defaults: &crate::app::launch_settings::LaunchDefaults) {
        self.values = Some(InstanceSettings::default().resolve(defaults));
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

    /// Applies a message. A change to a default leaves as [`Message::Change`] for the root, and the rest is the page's own.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Navigate(_) | Message::Change(_) => Task::none(),
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
                let mut vars = self.values.as_ref().map(|values| values.env.clone()).unwrap_or_default();
                edit.apply(&mut vars);
                Task::done(Message::Change(LaunchChange::Env(vars)))
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
        }
    }

    /// Takes a typed digit for one side of the resolution, and saves the pair once both sides are valid.
    fn resolution(&mut self, axis: Axis, text: &str) -> Task<Message> {
        let text = launch_rows::sanitize_dimension(text);
        match axis {
            Axis::Width => self.draft.width = Some(text),
            Axis::Height => self.draft.height = Some(text),
        }
        let Some(saved) = self.values.as_ref().map(|values| values.resolution) else {
            return Task::none();
        };
        let (width, height) = self.draft.texts(saved);
        match (launch_rows::parse_dimension(&width), launch_rows::parse_dimension(&height)) {
            (Some(width), Some(height)) => {
                self.draft = ResolutionDraft::default();
                let resolution = crate::app::launch_settings::Resolution { width, height };
                if resolution == saved {
                    Task::none()
                } else {
                    Task::done(Message::Change(LaunchChange::Resolution(resolution)))
                }
            }
            _ => Task::none(),
        }
    }

    /// The page: the heading, then the four launch sections, which scroll as one. The environment editor covers it while open.
    pub fn view(&self) -> Element<'_, Message> {
        let Some(values) = self.values.as_ref() else {
            return Space::new().into();
        };
        let shown = Shown {
            values,
            overrides: None,
            java: &self.java,
            ceiling_gb: self.ceiling_gb,
        };
        let on = |change: LaunchChange| Message::Change(change);
        let heading = column![
            crate::eyebrow!(lang::t("settings.eyebrow")),
            crate::display!(lang::t("settings.title")),
            crate::body!(lang::t("settings.defaults_intro")),
            row![
                TextButton::ghost(lang::t("common.home")).on_press(Message::Navigate(Page::Home)),
                TextButton::ghost(lang::t("settings.component_gallery")).on_press(Message::Navigate(Page::Showcase)),
            ]
            .spacing(12),
        ]
        .spacing(10);
        let sections = column![
            heading,
            block(
                Icon::lucide().cpu(),
                lang::t("instance.settings.section.java_memory"),
                lang::t("settings.defaults.subtitle.java_memory"),
                launch_rows::java_memory(shown, on),
            ),
            block(
                Icon::lucide().image(),
                lang::t("instance.settings.section.window"),
                lang::t("settings.defaults.subtitle.window"),
                launch_rows::window(shown, &self.draft, on, Message::Resolution),
            ),
            block(
                Icon::lucide().terminal(),
                lang::t("instance.settings.section.launch_args"),
                lang::t("settings.defaults.subtitle.launch_args"),
                launch_rows::launch_args(shown, on),
            ),
            block(
                Icon::lucide().settings(),
                lang::t("instance.settings.section.advanced"),
                lang::t("settings.defaults.subtitle.advanced"),
                launch_rows::advanced(shown, on, Message::OpenEnvironment),
            ),
        ]
        .spacing(40)
        .width(Fill);
        let page = container(sections)
            .width(Fill)
            .padding(Padding::new(0.0).left(28.0).right(28.0).top(28.0).bottom(40.0));
        let scrolled = scrollbar::scroll(page).width(Fill).height(Fill);

        if self.environment_open {
            let modal = launch_rows::environment_modal(&values.env, Message::Environment, Message::CloseEnvironment);
            return stack![scrolled, modal].width(Fill).height(Fill).into();
        }
        scrolled.into()
    }
}

/// One section: its heading, then its rows.
fn block<'a>(icon: &'static str, title: &'static str, subtitle: &'static str, body: Element<'a, Message>) -> Element<'a, Message> {
    column![section_header(icon, color::Primary::DEFAULT, title, subtitle), body]
        .spacing(16)
        .width(Fill)
        .into()
}
