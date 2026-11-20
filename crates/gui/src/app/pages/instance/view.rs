//! The instance page's layout: the masthead, the tab row, and the body of the tab it shows.

use std::collections::VecDeque;

use iced::widget::{Text, column, container, stack};
use iced::{Element, Fill, Padding, Size, alignment};

use super::{GUTTER, InstancePage, Message, Tab, content, hero, logs, mods, screenshots, snapshots};
use crate::app::components::Tabs;
use crate::app::components::instance_card::install_status;
use crate::app::components::scrollbar;
use crate::app::downloads::Pending;
use crate::app::lang;
use crate::app::launch::games::GameView;
use crate::app::pages::library::instance::{Instance, Loader};
use crate::app::theme::{color, fonts};
use crate::app::world_pins::WorldPins;

impl InstancePage {
    /// The page: fixed masthead, fixed tab row, and the tab body.
    /// The page. `busy` is the reason the instance cannot be updated now, when there is one, which the Versions tab shows.
    pub fn view<'a>(
        &'a self,
        viewport: Size,
        game: GameView<'a>,
        install: Option<&'a Pending>,
        pins: &'a WorldPins,
        busy: Option<&'static str>,
    ) -> Element<'a, Message> {
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
            self.tab_body(instance, game.console, pins, busy, viewport),
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
                    .color(color::text_headline()),
                Text::new(lang::t("instance.none_selected_hint"))
                    .size(12)
                    .font(fonts::Inter::REGULAR)
                    .color(color::text_muted()),
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
        let tabs = Tab::visible(self.instance.as_ref());
        let active = tabs.iter().position(|tab| *tab == self.tab).unwrap_or_default();

        container(
            Tabs::new(tabs.iter().map(|tab| tab.label()))
                .underline()
                .inset(GUTTER)
                .width(Fill)
                .active(active)
                .on_select(move |index| Message::TabSelected(tabs[index])),
        )
        .width(Fill)
        .padding(Padding::new(0.0).top(52.0))
        .into()
    }

    /// The Content tab: the switch over its sections, and the section it picked, which is the mod list or a pack list.
    fn content_body<'a>(&'a self, instance: &'a Instance) -> Element<'a, Message> {
        // The snapshots section is offered only while the instance keeps one. If the last one has gone, the mods show instead.
        let offer = !self.snapshots.is_empty();
        let section = match self.content_section {
            content::Section::Snapshots if !offer => content::Section::Mods,
            picked => picked,
        };
        let body: Element<'a, Message> = match section {
            content::Section::Snapshots => {
                let installed = snapshots::view::installed_index(self.mods.rows());
                snapshots::view::tab(&self.snapshots, &installed, instance.locked).map(Message::Snapshots)
            }
            picked => match picked.pack_kind() {
                None => self.mods.view(instance.loader != Loader::Vanilla).map(|message| match message {
                    mods::Message::Add => Message::AddMods,
                    other => Message::Mods(other),
                }),
                Some(kind) => self.packs.view(kind).map(Message::Packs),
            },
        };
        // The switch keeps the page's gutters. The mod list and the pack lists pad their own.
        let switch = container(content::switch(section, offer).map(Message::Content))
            .width(Fill)
            .padding(Padding::new(0.0).left(GUTTER).right(GUTTER));
        column![switch, body].spacing(18).width(Fill).height(Fill).into()
    }

    /// The selected tab's content, padded to the page gutter. The log panel and the mod list fill the height
    /// and scroll themselves, so their headers and filter bars stay put; every other tab scrolls as a page.
    fn tab_body<'a>(
        &'a self,
        instance: &'a Instance,
        console: Option<&'a VecDeque<logs::LogLine>>,
        pins: &'a WorldPins,
        busy: Option<&'static str>,
        viewport: Size,
    ) -> Element<'a, Message> {
        let content: Element<'a, Message> = match self.tab {
            Tab::Content => self.content_body(instance),
            Tab::Files => self.browser.view().map(Message::Browser),
            Tab::Versions => self.versions.view(busy, viewport).map(Message::Versions),
            Tab::Worlds => self.worlds.view(&instance.id, pins).map(Message::Worlds),
            Tab::Screenshots => screenshots::view(&self.screenshots).map(Message::Screenshots),
            Tab::Logs => self.logs.view(console).map(Message::Logs),
            Tab::Settings => self
                .settings
                .view(instance, &self.folder, crate::app::components::page::is_narrow(viewport.width))
                .map(Message::Settings),
        };
        // The mod list pads its own gutters, so its scrollbar sits in the page's gutter. The settings page scrolls its
        // sections beside its sidebar, so it takes the room the page leaves and pads its own bottom.
        let padding = match self.tab {
            Tab::Content => Padding::new(0.0).top(22.0),
            // The file list scrolls itself under its path bar, and keeps its own room at the end.
            Tab::Files => Padding::new(0.0).left(GUTTER).right(GUTTER).top(22.0).bottom(16.0),
            Tab::Settings => Padding::new(0.0).left(GUTTER).right(GUTTER).top(22.0),
            _ => Padding::new(0.0).left(GUTTER).right(GUTTER).top(22.0).bottom(40.0),
        };
        let padded = container(content).width(Fill).padding(padding);

        match self.tab {
            Tab::Logs | Tab::Content | Tab::Files | Tab::Settings => padded.height(Fill).into(),
            _ => scrollbar::scroll(padded).width(Fill).height(Fill).into(),
        }
    }
}
