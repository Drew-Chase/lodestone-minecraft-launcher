//! Settings — placeholder page for launcher preferences.

use crate::app::components::TextButton;
use crate::app::lang;
use crate::app::pages::Page;
use iced::widget::{column, container, row};
use iced::{Element, Fill, Padding};

/// State of the settings page.
#[derive(Debug, Default, Clone)]
pub struct Settings;

/// Messages produced by the settings page.
#[derive(Debug, Clone)]
pub enum Message {
    /// Requests navigation — the root model intercepts this before it
    /// reaches [`Settings::update`] (see the contract in `pages/mod.rs`).
    Navigate(Page),
}

impl Settings {
    pub fn update(&mut self, _message: Message) {
        // Only `Navigate` is produced today, and the root model intercepts
        // it before it reaches the page; nothing to do here yet.
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(
            column![
                crate::eyebrow!(lang::t("settings.eyebrow")),
                crate::display!(lang::t("settings.title")),
                crate::body!(lang::t("settings.intro")),
                crate::small!(lang::t("settings.empty")),
                row![
                    TextButton::ghost(lang::t("common.home")).on_press(Message::Navigate(Page::Home)),
                    TextButton::ghost(lang::t("settings.component_gallery")).on_press(Message::Navigate(Page::Showcase)),
                ]
                .spacing(12),
            ]
            .spacing(10)
            .width(Fill)
            .max_width(720),
        )
        .width(Fill)
        .height(Fill)
        .padding(Padding::new(0.0).left(24).right(24).top(32).bottom(64))
        .into()
    }
}
