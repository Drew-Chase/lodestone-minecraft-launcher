//! The sign-in area: the Microsoft button, the cancel action, the "or"
//! divider, and the offline/demo row beneath it.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, row};
use iced::{Element, Fill, Padding};

use crate::app::components::TextButton;
use crate::app::lang;
use crate::app::theme::{color as palette, fonts};

use super::widgets::{icon, ms_tiles, rule};
use super::{Flow, Login, Message};

impl Login {
    /// The Microsoft device-code area and its alternates, per flow state.
    /// The 18px rhythm matches the reference's collapsed margins.
    pub(super) fn sign_in_area(&self) -> Element<'_, Message> {
        let mut body = column![].spacing(18);
        if let Flow::Failed(error) = &self.flow {
            // Unmapped provider errors can be long — cap the card's copy so
            // the fixed-height layout never overflows.
            let shown = if error.chars().count() > 200 {
                format!("{}…", error.chars().take(200).collect::<String>())
            } else {
                error.clone()
            };
            body = body.push(
                Text::new(shown)
                    .size(12)
                    .font(fonts::Inter::MEDIUM)
                    .color(palette::Accents::PINK)
                    .line_height(1.4),
            );
        }
        match &self.flow {
            Flow::Code {
                user_code,
                verification_uri,
                copied,
            } => {
                body = body.push(self.code_panel(user_code, verification_uri, *copied));
                body = body.push(self.cancel_button());
            }
            Flow::Opening => {
                body = body.push(
                    self.microsoft_button(lang::t("login.opening_microsoft"), Message::SignInMicrosoft)
                        .disabled(true),
                );
            }
            // Offline mode asks for a username before entering.
            Flow::OfflinePrompt { username } => {
                body = body.push(self.offline_form(username));
            }
            // A stored sign-in is being restored silently.
            Flow::Restoring { username } => {
                body = body.push(self.restoring_panel(username));
            }
            // Idle and Failed both offer the button (retry after a failure).
            Flow::Idle | Flow::Failed(_) => {
                body = body.push(self.microsoft_button(lang::t("login.sign_in_microsoft"), Message::SignInMicrosoft));
                body = body.push(self.or_divider()).push(self.secondary_row());
            }
        }
        body.into()
    }

    /// The big "Continue with Microsoft" button.
    fn microsoft_button(&self, label: &str, on_press: Message) -> TextButton<'_, Message> {
        TextButton::content(
            row![
                Space::new().width(Fill),
                ms_tiles(),
                Text::new(label.to_string())
                    .size(14)
                    .font(fonts::Inter::BOLD)
                    .color(palette::Text::HEADLINE),
                Space::new().width(Fill),
                icon(Icon::lucide().chevron_right(), 16.0, palette::Text::HEADLINE, 0.45),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center)
            .width(Fill),
        )
        .width(Fill)
        .padding(Padding::new(0.0).left(18).right(14).top(14).bottom(14))
        .on_press(on_press)
    }

    /// Abandons an in-flight sign-in (the root invalidates the poll).
    fn cancel_button(&self) -> Element<'_, Message> {
        TextButton::content(
            row![
                Space::new().width(Fill),
                Text::new(lang::t("common.cancel"))
                    .size(13)
                    .font(fonts::Inter::MEDIUM)
                    .color(palette::Text::DEFAULT),
                Space::new().width(Fill),
            ]
            .align_y(iced::Alignment::Center)
            .width(Fill),
        )
        .width(Fill)
        .padding(Padding::new(0.0).top(12).bottom(12))
        .on_press(Message::CancelSignIn)
        .into()
    }

    /// The "OR" rule.
    fn or_divider(&self) -> Element<'_, Message> {
        row![
            rule(),
            Text::new(lang::t("login.or"))
                .size(10)
                .font(fonts::Inter::SEMI_BOLD)
                .color(palette::Text::PLACEHOLDER),
            rule(),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// Offline / Demo entrances — no Microsoft account required.
    fn secondary_row(&self) -> Element<'_, Message> {
        row![
            TextButton::content(
                container(
                    row![
                        icon(Icon::lucide().user(), 15.0, palette::Text::MUTED, 0.68),
                        Text::new(lang::t("login.offline_mode"))
                            .size(13)
                            .font(fonts::Inter::MEDIUM)
                            .color(palette::Text::DEFAULT),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                )
                .width(Fill)
                .center_x(Fill),
            )
            .width(Fill)
            .padding(Padding::new(0.0).top(12).bottom(12))
            .on_press(Message::OfflineMode),
            TextButton::content(
                container(
                    row![
                        icon(Icon::lucide().shield(), 15.0, palette::Text::MUTED, 0.68),
                        Text::new(lang::t("login.demo_mode"))
                            .size(13)
                            .font(fonts::Inter::MEDIUM)
                            .color(palette::Text::DEFAULT),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                )
                .width(Fill)
                .center_x(Fill),
            )
            .width(Fill)
            .padding(Padding::new(0.0).top(12).bottom(12))
            .on_press(Message::DemoMode),
        ]
        .spacing(10)
        .into()
    }
}
