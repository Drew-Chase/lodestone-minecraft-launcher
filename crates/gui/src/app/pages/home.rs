//! Home — the launcher's landing page.
//!
//! A placeholder: it proves the navigation contract by linking to the other
//! pages until real home-screen content lands.

use crate::app::components::TextButton;
use crate::app::pages::Page;
use iced::widget::{column, container, row};
use iced::{Element, Fill, Padding};

/// State of the home page.
#[derive(Debug, Default, Clone)]
pub struct Home;

/// Messages produced by the home page.
#[derive(Debug, Clone)]
pub enum Message {
    /// Requests navigation — the root model intercepts this before it
    /// reaches [`Home::update`] (see the contract in `pages/mod.rs`).
    Navigate(Page),
}

impl Home {
    pub fn update(&mut self, _message: Message) {
        // Only `Navigate` is produced today, and the root model intercepts
        // it before it reaches the page; nothing to do here yet.
    }

    pub fn view(&self) -> Element<'_, Message> {
        container(
            column![
                crate::eyebrow!("Launcher"),
                crate::display!("Home"),
                crate::body!("Welcome to Lodestone. Everything routes from here."),
                row![
                    TextButton::primary("Component gallery").on_press(Message::Navigate(Page::Showcase)),
                    TextButton::ghost("Settings").on_press(Message::Navigate(Page::Settings)),
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
