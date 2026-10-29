//! The panels that replace the sign-in buttons mid-flow: the device code,
//! the offline username form, and the silent-restore notice.

use aurora_leaf::Icon;
use iced::widget::{Space, Text, column, container, mouse_area, row, stack, text_input};
use iced::{Background, Border, Color, Element, Fill, Padding};
use iced_color_extension::rgb;

use crate::app::components::button;
use crate::app::components::input;
use crate::app::components::{ProgressBar, TextButton};
use crate::app::theme::{color as palette, fonts};

use super::widgets::{icon, rule_under_link};
use super::{Login, Message};

impl Login {
    /// The device code on screen while the poll runs.
    /// The device code on screen while the poll runs — compact, so the
    /// fixed-height card never overflows. Clicking the code copies it;
    /// clicking the link opens the browser.
    pub(super) fn code_panel(&self, user_code: &str, verification_uri: &str, copied: bool) -> Element<'_, Message> {
        let code_ink = if self.code_hover {
            palette::Text::HEADLINE
        } else {
            palette::Primary::DEFAULT
        };
        let link_ink = if self.link_hover {
            palette::Text::HEADLINE
        } else {
            palette::Primary::DEFAULT
        };
        container(
            column![
                row![
                    mouse_area(
                        container(
                            row![
                                Text::new(user_code.to_string())
                                    .size(20)
                                    .font(fonts::JetBrainsMono::SEMI_BOLD)
                                    .color(code_ink),
                                icon(
                                    Icon::lucide().copy(),
                                    14.0,
                                    palette::Text::HEADLINE,
                                    if self.code_hover { 1.0 } else { 0.0 },
                                ),
                            ]
                            .spacing(8)
                            .align_y(iced::Alignment::Center),
                        )
                        .padding(Padding::new(0.0).left(6).right(6).top(2).bottom(2))
                        .style(move |_| container::Style {
                            background: Some(Background::Color(if self.code_hover {
                                rgb!(34, 255, 132, 10%)
                            } else {
                                Color::TRANSPARENT
                            })),
                            border: Border {
                                radius: 6.0.into(),
                                ..Border::default()
                            },
                            ..container::Style::default()
                        }),
                    )
                    .on_press(Message::CopyCode { code: user_code.to_string() })
                    .on_enter(Message::CodeHover(true))
                    .on_exit(Message::CodeHover(false)),
                    Space::new().width(Fill),
                    mouse_area(
                        column![
                            row![
                                Text::new(verification_uri.replace("https://", "").replace("www.", ""))
                                    .size(11)
                                    .font(fonts::JetBrainsMono::REGULAR)
                                    .color(link_ink),
                                icon(
                                    Icon::lucide().external_link(),
                                    12.0,
                                    palette::Text::HEADLINE,
                                    if self.link_hover { 1.0 } else { 0.0 },
                                ),
                            ]
                            .spacing(4)
                            .align_y(iced::Alignment::Center),
                            rule_under_link(self.link_hover),
                        ]
                        .spacing(1),
                    )
                    .on_press(Message::OpenVerificationLink {
                        url: verification_uri.to_string()
                    })
                    .on_enter(Message::LinkHover(true))
                    .on_exit(Message::LinkHover(false))
                    .interaction(iced::mouse::Interaction::Pointer),
                ]
                .align_y(iced::Alignment::Center),
                ProgressBar::new(0.0).indeterminate(true).height(3.0).width(Fill),
                Text::new(if copied {
                    "Copied to clipboard — waiting for you to finish signing in…"
                } else {
                    "Click the code to copy it — waiting for you to finish signing in…"
                })
                .size(11)
                .font(fonts::Inter::REGULAR)
                .color(palette::Text::MUTED),
            ]
            .spacing(6),
        )
        .width(Fill)
        .padding(12)
        .style(|_| container::Style {
            background: Some(Background::Color(rgb!(34, 255, 132, 6%))),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: rgb!(34, 255, 132, 25%),
            },
            ..container::Style::default()
        })
        .into()
    }

    /// The offline username form — input, play, cancel.
    pub(super) fn offline_form(&self, username: &str) -> Element<'_, Message> {
        let ready = !username.trim().is_empty();
        if crate::app::probe_enabled() {
            tracing::info!("PROBE offline form: username={username:?} ready={ready}");
        }
        column![
            stack![
                text_input("Your username", username)
                    .on_input(Message::OfflineUsernameChanged)
                    .on_submit(Message::OfflineConfirm {
                        username: username.to_string()
                    })
                    .style(input::style)
                    .size(14)
                    .padding(Padding::new(0.0).left(38).right(12).top(12).bottom(12)),
                container(icon(Icon::lucide().user(), 16.0, palette::Text::MUTED, 0.68,))
                    .width(Fill)
                    .height(Fill)
                    .align_x(iced::alignment::Horizontal::Left)
                    .align_y(iced::alignment::Vertical::Center)
                    .padding(Padding::new(0.0).left(12)),
            ],
            row![
                TextButton::content(
                    row![
                        Space::new().width(Fill),
                        icon(Icon::material_symbols().play_arrow(), 14.0, palette::Primary::ON, 1.0),
                        Text::new("Play offline").size(13).font(fonts::Inter::MEDIUM).color(palette::Primary::ON),
                        Space::new().width(Fill),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .width(Fill),
                )
                .variant(button::Variant::Primary)
                .width(Fill)
                .padding(button::PRIMARY_PADDING)
                .disabled(!ready)
                .on_press(Message::OfflineConfirm {
                    username: username.to_string()
                }),
                TextButton::content(
                    row![
                        Space::new().width(Fill),
                        Text::new("Cancel").size(13).font(fonts::Inter::MEDIUM),
                        Space::new().width(Fill),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .width(Fill),
                )
                .width(Fill)
                .on_press(Message::Reset),
            ]
            .spacing(10),
        ]
        .spacing(10)
        .into()
    }

    /// The silent sign-in panel shown while a stored refresh token is
    /// being restored.
    pub(super) fn restoring_panel(&self, username: &str) -> Element<'_, Message> {
        container(
            column![
                row![
                    Text::new("Signing in as ")
                        .size(13)
                        .font(fonts::Inter::MEDIUM)
                        .color(palette::Text::MUTED),
                    Text::new(username.to_string())
                        .size(13)
                        .font(fonts::Inter::SEMI_BOLD)
                        .color(palette::Text::HEADLINE),
                    Text::new("…").size(13).font(fonts::Inter::MEDIUM).color(palette::Text::MUTED),
                ]
                .align_y(iced::Alignment::Center),
                ProgressBar::new(0.0).indeterminate(true).height(3.0).width(Fill),
            ]
            .spacing(8),
        )
        .width(Fill)
        .padding(12)
        .style(|_| container::Style {
            background: Some(Background::Color(rgb!(34, 255, 132, 6%))),
            border: Border {
                radius: 12.0.into(),
                width: 1.0,
                color: rgb!(34, 255, 132, 25%),
            },
            ..container::Style::default()
        })
        .into()
    }
}
