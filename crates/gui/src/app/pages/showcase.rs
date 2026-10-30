//! Showcase — a gallery of every core control, mounted in the app body so
//! each style-guide component can be verified visually (§07, §08).
//!
//! Deliberately self-contained: it owns its own state and messages, and can
//! be swapped for real pages without touching the components module.

use crate::app::components::language_picker::{LanguagePicker, PickerStyle};
use crate::app::components::scrollbar;
use crate::app::components::{Card, Dropdown, MenuItem, NavItem, NavRail, ProgressBar, PulseDot, Select, Tabs, TextButton, Toggle, input};
use crate::app::lang;
use crate::app::pages::Page;
use crate::app::theme::color;
use aurora_leaf::Icon;
use iced::widget::container::Style;
use iced::widget::{column, container, row, text_input};
use iced::{Alignment, Background, Color, Element, Fill, Gradient, Length, Padding, Theme, color, gradient};

/// Interactive state of the gallery.
#[derive(Debug, Default, Clone)]
pub struct Showcase {
    toggle_auto: bool,
    toggle_snapshots: bool,
    progress: f32,
    selected: Option<usize>,
    nav_active: usize,
    tab_active: usize,
    search: String,
    last_action: Option<&'static str>,
}

/// Messages produced by the gallery controls.
#[derive(Debug, Clone)]
pub enum Message {
    ToggleAuto(bool),
    ToggleSnapshots(bool),
    SimulateDownload,
    Selected(usize),
    NavSelected(usize),
    TabSelected(usize),
    SearchChanged(String),
    InstanceAction(usize),
    /// Requests navigation — the root model intercepts this before it
    /// reaches [`Showcase::update`] (see the contract in `pages/mod.rs`).
    Navigate(Page),
    /// The language picker chose a catalogue — the root applies and saves it.
    SelectLanguage(&'static str),
    /// For controls demonstrated without app behavior.
    Nop,
}

impl Showcase {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::ToggleAuto(value) => self.toggle_auto = value,
            Message::ToggleSnapshots(value) => self.toggle_snapshots = value,
            Message::SimulateDownload => self.progress = (self.progress + 0.17) % 1.0,
            Message::Selected(index) => self.selected = Some(index),
            // The root applies and saves the language before it reaches the gallery.
            Message::SelectLanguage(_) => {}
            Message::NavSelected(index) => self.nav_active = index,
            Message::TabSelected(index) => self.tab_active = index,
            Message::SearchChanged(value) => self.search = value,
            Message::InstanceAction(index) => {
                self.last_action = match index {
                    0 => Some(lang::t("showcase.menu.duplicate")),
                    2 => Some(lang::t("showcase.menu.delete")),
                    _ => None,
                };
            }
            Message::Navigate(_) => {
                // Intercepted by the root model before reaching the page.
            }
            Message::Nop => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        scrollbar::scroll(
            container(self.gallery())
                .width(Fill)
                .max_width(720)
                .padding(Padding::new(0.0).left(24).right(24).top(32).bottom(64)),
        )
        .width(Fill)
        .height(Fill)
        .into()
    }

    fn gallery(&self) -> Element<'_, Message> {
        column![
            self.header(),
            self.section(lang::t("showcase.section.buttons"), self.buttons()),
            self.section(lang::t("showcase.section.tabs"), self.tabs()),
            self.section(lang::t("showcase.section.inputs"), self.inputs()),
            self.section(lang::t("showcase.section.toggles"), self.toggles()),
            self.section(lang::t("showcase.section.progress"), self.progress_row()),
            self.section(lang::t("showcase.section.sidebar_nav"), self.nav()),
            self.section(lang::t("showcase.section.select"), self.menus()),
            self.section(lang::t("showcase.section.language_picker"), self.language_pickers()),
            self.section(lang::t("showcase.section.cards"), self.cards()),
        ]
        .spacing(20)
        .width(Fill)
        .into()
    }

    fn header(&self) -> Element<'_, Message> {
        column![crate::display!(lang::t("showcase.title")), crate::body!(lang::t("showcase.intro"))]
            .spacing(6)
            .into()
    }

    /// A section — an eyebrow label above a panel with the given content.
    fn section<'a>(&self, label: &'static str, content: impl Into<Element<'a, Message, Theme, iced::Renderer>>) -> Element<'a, Message> {
        column![crate::eyebrow!(label), panel(container(content.into()).width(Fill).padding(20)),]
            .spacing(8)
            .width(Fill)
            .into()
    }

    fn buttons(&self) -> Element<'_, Message> {
        row![
            TextButton::primary(lang::t("common.play")).on_press(Message::Nop),
            TextButton::ghost(lang::t("showcase.edit_instance")).on_press(Message::Nop),
            TextButton::ghost(lang::t("showcase.simulate_download")).on_press(Message::SimulateDownload),
            TextButton::primary(lang::t("showcase.disabled")).disabled(true),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .into()
    }

    fn tabs(&self) -> Element<'_, Message> {
        Tabs::new([
            lang::t("showcase.tabs.mods"),
            lang::t("showcase.tabs.resource_packs"),
            lang::t("showcase.tabs.worlds"),
            lang::t("showcase.tabs.logs"),
        ])
        .active(self.tab_active)
        .on_select(Message::TabSelected)
        .into()
    }

    fn inputs(&self) -> Element<'_, Message> {
        column![
            text_input(lang::t("showcase.search_placeholder"), &self.search)
                .on_input(Message::SearchChanged)
                .style(input::style)
                .size(13.0)
                .padding(10)
                .width(Fill),
        ]
        .spacing(8)
        .into()
    }

    fn toggles(&self) -> Element<'_, Message> {
        row![
            labeled(
                lang::t("showcase.auto_update"),
                Toggle::new(self.toggle_auto).on_toggle_with(Message::ToggleAuto),
            ),
            labeled(
                lang::t("showcase.snapshots"),
                Toggle::new(self.toggle_snapshots).on_toggle_with(Message::ToggleSnapshots),
            ),
            labeled(lang::t("showcase.online"), PulseDot::new()),
            labeled(lang::t("showcase.stale"), PulseDot::with_color(color::Accents::AMBER).pulsing(false),),
        ]
        .spacing(32)
        .align_y(Alignment::Center)
        .into()
    }

    fn progress_row(&self) -> Element<'_, Message> {
        column![
            row![
                crate::body!(lang::t("showcase.downloading")),
                spacer(),
                crate::code!(format!("{}%", (self.progress * 100.0) as usize)),
            ]
            .width(Fill),
            ProgressBar::new(self.progress).height(6.0),
            crate::small!(lang::t("showcase.indeterminate")),
            ProgressBar::new(0.0).indeterminate(true).height(6.0),
        ]
        .spacing(10)
        .width(Fill)
        .into()
    }

    fn nav(&self) -> Element<'_, Message> {
        row![
            NavRail::new([
                NavItem::new(Icon::lucide().house()),
                NavItem::new(Icon::lucide().compass()),
                NavItem::new(Icon::lucide().download()),
                NavItem::new(Icon::lucide().settings()),
            ])
            .active(self.nav_active)
            .on_select(Message::NavSelected),
            spacer(),
            column![crate::h3!(lang::t("showcase.slide_title")), crate::body!(lang::t("showcase.slide_body"))].spacing(6),
        ]
        .width(Fill)
        .align_y(Alignment::Center)
        .into()
    }

    fn menus(&self) -> Element<'_, Message> {
        let action_label = self.last_action.unwrap_or("…");
        row![
            column![Select::new(["Vanilla", "Fabric", "Quilt", "NeoForge"], self.selected).on_select(Message::Selected),].width(Fill),
            Dropdown::new(Icon::lucide().ellipsis_vertical())
                .item(MenuItem::Action {
                    label: String::from(lang::t("showcase.menu.duplicate_instance")),
                    icon: Some(Icon::lucide().copy()),
                    danger: false,
                })
                .separator()
                .item(MenuItem::Action {
                    label: String::from(lang::t("showcase.menu.delete_instance")),
                    icon: Some(Icon::lucide().trash()),
                    danger: true,
                })
                .on_select(Message::InstanceAction),
            column![crate::small!(lang::t("showcase.last_action")), crate::code!(action_label),],
        ]
        .spacing(16)
        .align_y(Alignment::Center)
        .width(Fill)
        .into()
    }

    /// The language picker in both looks: the chip the login page uses, and the field a settings
    /// row uses. Both route their choice through the root, as the login page does.
    fn language_pickers(&self) -> Element<'_, Message> {
        row![
            column![
                crate::small!(lang::t("showcase.language_chip")),
                LanguagePicker::new(Message::SelectLanguage).style(PickerStyle::Chip),
            ]
            .spacing(8),
            column![
                crate::small!(lang::t("showcase.language_field")),
                LanguagePicker::new(Message::SelectLanguage),
            ]
            .spacing(8),
        ]
        .spacing(32)
        .align_y(Alignment::Start)
        .width(Fill)
        .into()
    }

    fn cards(&self) -> Element<'_, Message> {
        row![
            Card::new(
                Some(cover(color!(0x2D8A3F), color!(0x0F3017), 120.0)),
                column![
                    row![crate::h3!("Vanilla+"), spacer(), crate::code!("1.21.4"),].width(Fill),
                    crate::small!(lang::t("showcase.played_2h")),
                ]
                .spacing(6),
            )
            .width(Fill),
            Card::new(
                Some(cover(color!(0xFF6B3D), color!(0x3D0A08), 120.0)),
                column![
                    row![crate::h3!("Better Nether"), spacer(), crate::code!("Fabric"),].width(Fill),
                    crate::small!(lang::t("showcase.played_yesterday")),
                ]
                .spacing(6),
            )
            .width(Fill),
        ]
        .spacing(14)
        .width(Fill)
        .into()
    }
}

/// Flexible horizontal filler.
fn spacer() -> iced::widget::Space {
    iced::widget::Space::new().width(Fill)
}

fn labeled<'a>(label: &'static str, control: impl Into<Element<'a, Message, Theme, iced::Renderer>>) -> Element<'a, Message> {
    row![control.into(), crate::small!(label)].spacing(10).align_y(Alignment::Center).into()
}

/// Panel surface — the guide's elevation-1 card fill.
fn panel<'a>(content: impl Into<Element<'a, Message, Theme, iced::Renderer>>) -> Element<'a, Message> {
    container(content.into())
        .width(Fill)
        .style(|_| Style {
            background: Some(Background::Gradient(Gradient::Linear(
                gradient::Linear::new(std::f32::consts::FRAC_PI_2)
                    .add_stop(0.0, color!(0x0F1114))
                    .add_stop(1.0, color!(0x0C0D10)),
            ))),
            border: iced::Border {
                radius: 18.0.into(),
                width: 1.0,
                color: color::Surface::LINE,
            },
            ..Style::default()
        })
        .into()
}

/// A biome-style gradient cover tile.
fn cover(top: Color, bottom: Color, height: f32) -> Element<'static, Message> {
    container(iced::widget::Space::new().width(Fill).height(Length::Fixed(height)))
        .width(Fill)
        .style(move |_| Style {
            background: Some(Background::Gradient(Gradient::Linear(
                gradient::Linear::new(std::f32::consts::FRAC_PI_2)
                    .add_stop(0.0, top)
                    .add_stop(1.0, bottom),
            ))),
            ..Style::default()
        })
        .into()
}

#[cfg(test)]
#[path = "../../../tests/app/pages/showcase.rs"]
mod tests;
