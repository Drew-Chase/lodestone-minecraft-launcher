//! The instance detail page — the immersive per-instance view from
//! `04-instance.png`.
//!
//! The page is a fixed masthead plus tab row over one content region. Each tab
//! lives in its own module with its own state, message and view; this file
//! routes messages and lays out the shell. Tab data is seeded by [`demo`] until
//! a real instance manager exists. The instance model's own data (name, loader,
//! version, mods, playtime, folder, biome) is real.

mod demo;
mod format;
mod hero;
mod logs;
mod mods;
mod overview;
mod screenshots;
mod settings;
mod worlds;

use chrono::Local;
use iced::widget::{Text, column, container, scrollable, stack};
use iced::{Element, Fill, Padding, Size, alignment};

use crate::app::components::Tabs;
use crate::app::pages::Page;
use crate::app::pages::library::instance::Instance;
use crate::app::theme::{color, fonts};

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
            Tab::Overview => "Overview",
            Tab::Mods => "Mods",
            Tab::Worlds => "Worlds",
            Tab::Screenshots => "Screenshots",
            Tab::Logs => "Logs",
            Tab::Settings => "Settings",
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
    /// Favourites the instance (display state only for now).
    ToggleFavorite,
    /// Reveals the instance folder (the root owns IO).
    RevealFolder,
    /// The kebab menu — display-only until the instance actions land.
    More,
    /// Pauses or resumes the running session (display state only).
    TogglePause,
    /// Requests navigation — the root intercepts this before the page's
    /// `update` runs (the `<Link>` of this framework).
    Navigate(Page),
    Mods(mods::Message),
    Worlds(worlds::Message),
    Logs(logs::Message),
    Settings(settings::Message),
}

/// The instance detail page state.
#[derive(Debug, Default)]
pub struct InstancePage {
    instance: Option<Instance>,
    tab: Tab,
    favorite: bool,
    paused: bool,
    mods: mods::State,
    worlds: worlds::State,
    screenshots: screenshots::State,
    logs: logs::State,
    settings: settings::State,
}

impl InstancePage {
    /// Points the page at `instance`, resetting transient controls and
    /// rebuilding every tab's data for it.
    pub fn show(&mut self, instance: Instance) {
        let now = Local::now();
        self.mods = mods::State::for_instance(&instance);
        self.worlds = worlds::State::for_instance(&instance, now);
        self.screenshots = screenshots::State::for_instance(&instance);
        self.logs = logs::State::for_instance(&instance);
        self.settings = settings::State::for_instance(&instance);
        self.instance = Some(instance);
        self.tab = Tab::Overview;
        self.paused = false;
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::TabSelected(tab) => self.tab = tab,
            Message::ToggleFavorite => self.favorite = !self.favorite,
            Message::TogglePause => self.paused = !self.paused,
            Message::Mods(message) => self.mods.update(message),
            Message::Worlds(message) => self.worlds.update(message),
            Message::Logs(message) => self.logs.update(message),
            Message::Settings(message) => self.settings.update(message),
            // Navigation is intercepted by the root; the remaining actions
            // are display-only until the instance manager lands.
            Message::Back | Message::RevealFolder | Message::More | Message::Navigate(_) => {}
        }
    }

    /// The page: fixed masthead, fixed tab row, and the tab body.
    pub fn view(&self, viewport: Size) -> Element<'_, Message> {
        let Some(instance) = &self.instance else {
            return self.empty_state();
        };

        let controls = hero::Controls {
            favorite: self.favorite,
            paused: self.paused,
        };
        let page = column![hero::hero(instance, controls, viewport), self.tab_bar(), self.tab_body(instance),]
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
                Text::new("No instance selected")
                    .size(18)
                    .font(fonts::Inter::EXTRA_BOLD)
                    .color(color::Text::HEADLINE),
                Text::new("Choose an instance in the Library to open its detail view.")
                    .size(12)
                    .font(fonts::Inter::REGULAR)
                    .color(color::Text::MUTED),
                crate::app::components::TextButton::ghost("Back to Library").on_press(Message::Back),
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
    fn tab_body<'a>(&'a self, instance: &'a Instance) -> Element<'a, Message> {
        let content: Element<'a, Message> = match self.tab {
            Tab::Overview => overview::view(instance),
            Tab::Mods => self.mods.view().map(Message::Mods),
            Tab::Worlds => self.worlds.view().map(Message::Worlds),
            Tab::Screenshots => screenshots::view(&self.screenshots),
            Tab::Logs => self.logs.view().map(Message::Logs),
            Tab::Settings => self.settings.view(instance).map(Message::Settings),
        };
        let padded = container(content)
            .width(Fill)
            .padding(Padding::new(0.0).left(GUTTER).right(GUTTER).top(22.0).bottom(40.0));

        match self.tab {
            Tab::Logs => padded.height(Fill).into(),
            _ => scrollable(padded).width(Fill).height(Fill).into(),
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
