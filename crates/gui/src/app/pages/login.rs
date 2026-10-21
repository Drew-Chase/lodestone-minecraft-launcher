//! Login — the gate in front of the launcher's primary navigation.
//!
//! Recreates the reference artboard `01 · Login`: the card (logo, serif
//! headline, Microsoft device-code sign-in, offline/demo entrances) floats
//! over the [`LoginBackdrop`](crate::app::components::backdrop::LoginBackdrop),
//! which the root model mounts full-bleed behind a transparent titlebar.
//!
//! The Microsoft button drives the two-phase device-code flow from
//! `crates/auth`: phase one fetches the code (root task), the card shows it,
//! and the poll's outcome comes back through the root. This page only owns
//! the *display* state machine — every message with app-level effect is
//! intercepted by the root (see the contract in `pages/mod.rs`).

use std::sync::OnceLock;

use aurora_leaf::Icon;
use iced::font::Family;
use iced::widget::svg::Handle;
use iced::widget::{Space, Text, column, container, row, stack, svg};
use iced::{Background, Border, Color, Element, Fill, Font, Length, Padding, Radians, Shadow, Vector, color, gradient};
use iced_color_extension::rgb;
use lodestone_minecraft_authentication::{DeviceCodeStart, Session};

use crate::app::LodestoneLauncher;
use crate::app::components::backdrop::logo_glow_handle;
use crate::app::components::{ProgressBar, PulseDot, TextButton};
use crate::app::pages::Page;
use crate::app::theme::{color as palette, fonts};

/// The Lodestone mark from the app's icon asset.
const LOGO_SVG: &str = include_str!("../../../../../res/icon/icon.svg");

/// The headline's serif — the reference renders the welcome line in a serif
/// face, which the guide's Inter/JetBrains families don't cover.
const SERIF_BOLD: Font = Font {
    family: Family::Serif,
    weight: Font::DEFAULT.weight,
    ..Font::DEFAULT
};

/// The signed-in flow's display state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum Flow {
    /// Nothing happening — the standard card.
    #[default]
    Idle,
    /// The device-code request is in flight.
    Opening,
    /// A code is on screen and the sign-in poll is running.
    Code {
        /// The short code the user types at `verification_uri`.
        user_code: String,
        /// Where to enter it, e.g. `https://www.microsoft.com/link`.
        verification_uri: String,
    },
    /// The last attempt failed with a user-presentable message; the card
    /// offers a retry.
    Failed(String),
}

/// State of the login page.
#[derive(Debug, Clone, Default)]
pub struct Login {
    flow: Flow,
}

/// Messages produced by the login page.
#[derive(Debug, Clone)]
pub enum Message {
    /// Requests navigation — the root model intercepts this before it
    /// reaches [`Login::update`] (see the contract in `pages/mod.rs`).
    Navigate(Page),
    /// "Continue with Microsoft" — the root starts the device-code flow.
    SignInMicrosoft,
    /// "Offline Mode" — the root enters the launcher without an account.
    OfflineMode,
    /// "Demo Mode" — the root enters the launcher without an account.
    DemoMode,
    /// "Learn more ↗" — informational link.
    LearnMore,
    /// Internal — the root forwards this when the device code arrives.
    Opening,
    /// Internal — a device code is ready to be shown.
    CodeReady {
        /// The short code the user types.
        user_code: String,
        /// Where to enter it.
        verification_uri: String,
    },
    /// Internal — the sign-in ended in a user-presentable failure.
    SignInFailed(String),
    /// Internal — drop any flow state and return to the standard card.
    Reset,
    /// Internal — phase one of the device flow resolved (or failed with a
    /// user-presentable message). Root-only: carries the pollable state.
    DeviceCodeStarted(Result<DeviceCodeStart, String>),
    /// Internal — the sign-in poll resolved with a session or a
    /// user-presentable failure. Root-only.
    SignInFinished(Result<Session, String>),
}

impl Login {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Opening => self.flow = Flow::Opening,
            Message::CodeReady { user_code, verification_uri } => {
                self.flow = Flow::Code { user_code, verification_uri };
            }
            Message::SignInFailed(error) => self.flow = Flow::Failed(error),
            Message::Reset => self.flow = Flow::Idle,
            // `Navigate` is intercepted by the root model before reaching the
            // page; the actions and the task-callback messages are the
            // root's to handle.
            Message::Navigate(_)
            | Message::SignInMicrosoft
            | Message::OfflineMode
            | Message::DemoMode
            | Message::LearnMore
            | Message::DeviceCodeStarted(_)
            | Message::SignInFinished(_) => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            // Language chip — top left, over the backdrop.
            container(self.language_chip()).width(Fill).padding(Padding::new(0.0).left(24).top(12)),
            // The card, centered in the remaining space.
            container(self.card()).width(Fill).height(Fill).center(Length::Fill),
            self.status_bar(),
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    /// The "English · US" chip — decorative until localization lands.
    fn language_chip(&self) -> Element<'_, Message> {
        container(
            row![
                icon(Icon::lucide().globe(), 14.0, Some(palette::Text::MUTED)),
                Text::new("English · US")
                    .size(12)
                    .font(fonts::Inter::MEDIUM)
                    .color(palette::Text::DEFAULT),
                icon(Icon::lucide().chevron_down(), 12.0, Some(palette::Text::MUTED)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .padding(Padding::new(0.0).left(16).right(12).top(10).bottom(10))
        .style(|_| container::Style {
            background: Some(Background::Color(rgb!(0, 0, 0, 40%))),
            border: Border {
                radius: 12.0.into(),
                ..Border::default()
            },
            ..container::Style::default()
        })
        .into()
    }

    /// The floating sign-in card.
    fn card(&self) -> Element<'_, Message> {
        container(stack![
            // Green glow washing the top-right corner of the card.
            container(svg(logo_glow_handle().clone()).width(220).height(220))
                .width(Fill)
                .height(Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .align_y(iced::alignment::Vertical::Top),
            self.card_content(),
        ])
        .width(440)
        .style(|_| container::Style {
            background: Some(Background::Gradient(
                gradient::Linear::new(Radians(std::f32::consts::FRAC_PI_2))
                    .add_stop(0.0, rgb!(21, 24, 28, 85%))
                    .add_stop(1.0, rgb!(14, 16, 18, 92%))
                    .into(),
            )),
            border: Border {
                radius: 20.0.into(),
                width: 1.0,
                color: rgb_line(),
            },
            shadow: Shadow {
                color: rgb!(0, 0, 0, 60%),
                offset: Vector::new(0.0, 30.0),
                blur_radius: 80.0,
            },
            ..container::Style::default()
        })
        .into()
    }

    /// Everything inside the card.
    fn card_content(&self) -> Element<'_, Message> {
        column![
            self.logo_row(),
            self.headline(),
            Text::new("Sign in to sync your worlds, instances, and friends across devices.")
                .size(13)
                .font(fonts::Inter::REGULAR)
                .color(palette::Text::MUTED)
                .line_height(1.5),
            self.sign_in_area(),
            self.footer(),
        ]
        .spacing(14)
        .padding(Padding::new(0.0).left(36).right(36).top(40).bottom(32))
        .into()
    }

    /// The Lodestone mark with its glow halo, plus the wordmark.
    fn logo_row(&self) -> Element<'_, Message> {
        row![
            stack![
                container(svg(logo_glow_handle().clone()).width(96).height(96))
                    .width(Fill)
                    .height(Fill)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center),
                svg(logo_handle().clone()).width(52).height(52),
            ]
            .width(52)
            .height(52),
            column![
                Text::new("Lodestone").size(22).font(SERIF_BOLD).color(palette::Text::HEADLINE),
                mono(format!("THIRD-PARTY LAUNCHER · v{}", LodestoneLauncher::VERSION), 11.0),
            ]
            .spacing(2),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// "Welcome back, crafter" — the one serif moment in the app.
    fn headline(&self) -> Element<'_, Message> {
        row![
            Text::new("Welcome back, ").size(26).font(SERIF_BOLD).color(palette::Text::HEADLINE),
            Text::new("crafter").size(26).font(SERIF_BOLD).color(palette::Primary::DEFAULT),
        ]
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// The Microsoft device-code area and its alternates, per flow state.
    fn sign_in_area(&self) -> Element<'_, Message> {
        let mut body = column![];
        if let Flow::Failed(error) = &self.flow {
            body = body.push(
                Text::new(error.clone())
                    .size(12)
                    .font(fonts::Inter::MEDIUM)
                    .color(palette::Accents::PINK)
                    .line_height(1.4),
            );
        }
        match &self.flow {
            Flow::Code { user_code, verification_uri } => {
                body = body.push(self.code_panel(user_code, verification_uri));
            }
            Flow::Opening => {
                body = body.push(self.microsoft_button("Opening Microsoft…", Message::SignInMicrosoft).disabled(true));
            }
            // Idle and Failed both offer the button (retry after a failure).
            Flow::Idle | Flow::Failed(_) => {
                body = body.push(self.microsoft_button("Continue with Microsoft", Message::SignInMicrosoft));
            }
        }
        body.push(self.or_divider()).push(self.secondary_row()).into()
    }

    /// The big "Continue with Microsoft" button.
    fn microsoft_button(&self, label: &str, on_press: Message) -> TextButton<'_, Message> {
        TextButton::content(
            row![
                ms_tiles(),
                Text::new(label.to_string())
                    .size(14)
                    .font(fonts::Inter::BOLD)
                    .color(palette::Text::HEADLINE),
                Space::new().width(Fill),
                icon(Icon::lucide().chevron_right(), 16.0, Some(palette::Text::PLACEHOLDER)),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center)
            .width(Fill),
        )
        .width(Fill)
        .padding(Padding::new(0.0).left(18).right(14).top(14).bottom(14))
        .on_press(on_press)
    }

    /// The device code on screen while the poll runs.
    fn code_panel(&self, user_code: &str, verification_uri: &str) -> Element<'_, Message> {
        container(
            column![
                row![
                    Text::new("Open ").size(12).font(fonts::Inter::MEDIUM).color(palette::Text::MUTED),
                    Text::new(verification_uri.to_string())
                        .size(12)
                        .font(fonts::JetBrainsMono::SEMI_BOLD)
                        .color(palette::Primary::DEFAULT),
                    Text::new(" and enter").size(12).font(fonts::Inter::MEDIUM).color(palette::Text::MUTED),
                ]
                .spacing(4),
                Text::new(user_code.to_string())
                    .size(24)
                    .font(fonts::JetBrainsMono::SEMI_BOLD)
                    .color(palette::Text::HEADLINE),
                ProgressBar::new(0.0).indeterminate(true).height(4.0).width(Fill),
                Text::new("Waiting for you to finish signing in…")
                    .size(12)
                    .font(fonts::Inter::REGULAR)
                    .color(palette::Text::MUTED),
            ]
            .spacing(8),
        )
        .width(Fill)
        .padding(16)
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

    /// The "OR" rule.
    fn or_divider(&self) -> Element<'_, Message> {
        row![rule(), mono("OR", 10.0), rule()].spacing(12).align_y(iced::Alignment::Center).into()
    }

    /// Offline / Demo entrances — no Microsoft account required.
    fn secondary_row(&self) -> Element<'_, Message> {
        row![
            TextButton::content(
                row![
                    icon(Icon::lucide().user(), 15.0, Some(palette::Text::MUTED)),
                    Text::new("Offline Mode")
                        .size(13)
                        .font(fonts::Inter::MEDIUM)
                        .color(palette::Text::DEFAULT),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .width(Fill)
            .padding(Padding::new(0.0).top(12).bottom(12))
            .on_press(Message::OfflineMode),
            TextButton::content(
                row![
                    icon(Icon::lucide().shield(), 15.0, Some(palette::Text::MUTED)),
                    Text::new("Demo Mode").size(13).font(fonts::Inter::MEDIUM).color(palette::Text::DEFAULT),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .width(Fill)
            .padding(Padding::new(0.0).top(12).bottom(12))
            .on_press(Message::DemoMode),
        ]
        .spacing(10)
        .into()
    }

    /// The card's legal line.
    fn footer(&self) -> Element<'_, Message> {
        row![
            mono("Not affiliated with Mojang or Microsoft", 11.0),
            Space::new().width(Fill),
            TextButton::content(
                Text::new("Learn more ↗")
                    .size(11)
                    .font(fonts::Inter::MEDIUM)
                    .color(palette::Primary::DEFAULT),
            )
            .padding(Padding::new(0.0).left(4).right(4).top(2).bottom(2))
            .on_press(Message::LearnMore),
        ]
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// The service status line pinned to the bottom of the screen. The ping
    /// and region are decorative statics until the services layer exists.
    fn status_bar(&self) -> Element<'_, Message> {
        row![
            PulseDot::new().size(6.0).pulsing(true),
            mono("AUTH SERVICES ONLINE", 11.0),
            mono("·", 11.0),
            mono("124ms · NA-EAST", 11.0),
            Space::new().width(Fill),
            mono("1.20.4 · 1.20.1 · 1.19.4 · 1.18.2 available", 11.0),
        ]
        .spacing(14)
        .align_y(iced::Alignment::Center)
        .padding(Padding::new(0.0).left(24).right(24).bottom(18))
        .width(Fill)
        .into()
    }
}

/// The logo handle — built once from the embedded icon asset.
fn logo_handle() -> &'static Handle {
    static LOGO: OnceLock<Handle> = OnceLock::new();
    LOGO.get_or_init(|| Handle::from_memory(LOGO_SVG.as_bytes()))
}

/// Mono text in the status bar's style — `--ink-3`, no line-height games.
fn mono(content: impl Into<String>, size: f32) -> Text<'static> {
    Text::new(content.into())
        .font(fonts::JetBrainsMono::REGULAR)
        .size(size)
        .color(palette::Text::PLACEHOLDER)
}

/// An SVG icon from a lucide markup string, optionally tinted.
fn icon(markup: &'static str, size: f32, tint: Option<Color>) -> Element<'static, Message> {
    let base = svg(Handle::from_memory(markup.as_bytes())).width(size).height(size);
    match tint {
        Some(tint) => base.style(move |_, _| svg::Style { color: Some(tint) }).into(),
        None => base.into(),
    }
}

/// The card's hairline border color — `--line` (white 7%) bumped to 9% per
/// the artboard.
fn rgb_line() -> Color {
    rgb!(255, 255, 255, 9%)
}

/// A 1px horizontal rule in `--line`.
fn rule() -> Element<'static, Message> {
    container(Space::new().height(1).width(Fill))
        .width(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(palette::Surface::LINE)),
            ..container::Style::default()
        })
        .into()
}

/// The four Microsoft logo tiles, 2×2 with 2px gutters.
fn ms_tiles() -> Element<'static, Message> {
    let tile = |c: Color| {
        container(Space::new().width(7).height(7)).style(move |_| container::Style {
            background: Some(Background::Color(c)),
            ..container::Style::default()
        })
    };
    column![
        row![tile(color!(0xF25022)), tile(color!(0x7FBA00))].spacing(2),
        row![tile(color!(0x00A4EF)), tile(color!(0xFFB900))].spacing(2),
    ]
    .spacing(2)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flow_moves_through_its_states() {
        let mut login = Login::default();
        assert_eq!(login.flow, Flow::Idle);

        login.update(Message::Opening);
        assert_eq!(login.flow, Flow::Opening);

        login.update(Message::CodeReady {
            user_code: "ABCD-EFGH".into(),
            verification_uri: "https://www.microsoft.com/link".into(),
        });
        assert_eq!(
            login.flow,
            Flow::Code {
                user_code: "ABCD-EFGH".into(),
                verification_uri: "https://www.microsoft.com/link".into()
            }
        );

        login.update(Message::SignInFailed("the sign-in request was denied".into()));
        assert_eq!(login.flow, Flow::Failed("the sign-in request was denied".into()));

        login.update(Message::Reset);
        assert_eq!(login.flow, Flow::Idle);
    }

    #[test]
    fn actions_are_the_roots_to_handle() {
        // The page never acts on its own action messages — the root
        // intercepts them (see the contract in `pages/mod.rs`).
        let mut login = Login::default();
        for message in [Message::SignInMicrosoft, Message::OfflineMode, Message::DemoMode, Message::LearnMore] {
            login.update(message.clone());
            assert_eq!(login.flow, Flow::Idle);
        }
        login.update(Message::Navigate(Page::Home));
        assert_eq!(login.flow, Flow::Idle);
    }
}
