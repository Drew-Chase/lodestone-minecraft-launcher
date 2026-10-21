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
use iced::widget::svg::Handle;
use iced::widget::{Space, Text, column, container, mouse_area, row, stack, svg};
use iced::{Background, Border, Color, Element, Fill, Length, Padding, Radians, Shadow, Vector, color, gradient};
use iced_color_extension::rgb;
use lodestone_minecraft_authentication::{DeviceCodeStart, Session};

use crate::app::LodestoneLauncher;
use crate::app::components::backdrop::logo_glow_handle;
use crate::app::components::{ProgressBar, PulseDot, TextButton};
use crate::app::pages::Page;
use crate::app::theme::{color as palette, fonts};

/// The Lodestone mark from the app's icon asset.
const LOGO_SVG: &str = include_str!("../../../../../res/icon/icon.svg");

/// The signed-in flow's display state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
enum Flow {
    /// Nothing happening — the standard card.
    #[default]
    Idle,
    /// The device-code request is in flight.
    Opening,
    /// A code is on screen and the sign-in poll is running. The alternates
    /// (OR + Offline/Demo) are hidden while signing in; a cancel button
    /// takes their place.
    Code {
        /// The short code the user types at `verification_uri`.
        user_code: String,
        /// Where to enter it, e.g. `https://www.microsoft.com/link`.
        verification_uri: String,
        /// The code was clicked and copied to the clipboard.
        copied: bool,
    },
    /// The last attempt failed with a user-presentable message; the card
    /// offers a retry.
    Failed(String),
}

/// State of the login page.
#[derive(Debug, Clone, Default)]
pub struct Login {
    flow: Flow,
    /// Hover state of the code (copy) region in the code panel.
    code_hover: bool,
    /// Hover state of the verification-link region.
    link_hover: bool,
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
    /// The device code was clicked — the root copies it to the clipboard.
    CopyCode { code: String },
    /// The verification link was clicked — the root opens the browser.
    OpenVerificationLink { url: String },
    /// "Cancel" — the root abandons the in-flight sign-in.
    CancelSignIn,
    /// Hover entered/exited the code (copy) region.
    CodeHover(bool),
    /// Hover entered/exited the verification-link region.
    LinkHover(bool),
    /// Internal — the root forwards this when the device code arrives.
    Opening,
    /// Internal — a device code is ready to be shown.
    CodeReady {
        /// The short code the user types.
        user_code: String,
        /// Where to enter it.
        verification_uri: String,
    },
    /// Internal — the code was copied (updates the panel hint).
    CodeCopied,
    /// Internal — the sign-in ended in a user-presentable failure.
    SignInFailed(String),
    /// Internal — drop any flow state and return to the standard card.
    Reset,
    /// Internal — phase one of the device flow resolved (or failed with a
    /// user-presentable message). Root-only: carries the pollable state and
    /// the generation that spawned it (stale results are dropped).
    DeviceCodeStarted(u64, Result<DeviceCodeStart, String>),
    /// Internal — the sign-in poll resolved with a session or a
    /// user-presentable failure. Root-only.
    SignInFinished(u64, Result<Session, String>),
}

impl Login {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Opening => self.flow = Flow::Opening,
            Message::CodeReady { user_code, verification_uri } => {
                self.flow = Flow::Code {
                    user_code,
                    verification_uri,
                    copied: false,
                };
            }
            Message::CodeCopied => {
                if let Flow::Code { copied, .. } = &mut self.flow {
                    *copied = true;
                }
            }
            Message::SignInFailed(error) => self.flow = Flow::Failed(error),
            Message::Reset => {
                self.flow = Flow::Idle;
                self.code_hover = false;
                self.link_hover = false;
            }
            Message::CodeHover(hovered) => self.code_hover = hovered,
            Message::LinkHover(hovered) => self.link_hover = hovered,
            // `Navigate` is intercepted by the root model before reaching the
            // page; the actions, the copy/open effects, and the task-callback
            // messages are the root's to handle.
            Message::Navigate(_)
            | Message::SignInMicrosoft
            | Message::OfflineMode
            | Message::DemoMode
            | Message::LearnMore
            | Message::CopyCode { .. }
            | Message::OpenVerificationLink { .. }
            | Message::CancelSignIn
            | Message::DeviceCodeStarted(_, _)
            | Message::SignInFinished(_, _) => {}
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        column![
            // Top chrome overlays: language chip left, brand centered.
            container(self.top_bar())
                .width(Fill)
                .padding(Padding::new(0.0).left(24).right(24).top(12)),
            // The card, centered in the remaining space.
            container(self.card()).width(Fill).height(Fill).center(Length::Fill),
            self.status_bar(),
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    /// Language chip left, window brand centered — the reference's top edge.
    fn top_bar(&self) -> Element<'_, Message> {
        stack![
            container(self.language_chip()).width(Fill).align_x(iced::alignment::Horizontal::Left),
            container(mono(format!("LODESTONE · v{}", LodestoneLauncher::VERSION), 11.0))
                .width(Fill)
                .align_x(iced::alignment::Horizontal::Center),
        ]
        .into()
    }

    /// The "English · US" chip — decorative until localization lands.
    fn language_chip(&self) -> Element<'_, Message> {
        container(
            row![
                icon(Icon::lucide().globe(), 14.0, palette::Text::MUTED, 0.68),
                Text::new("English · US")
                    .size(12)
                    .font(fonts::Inter::MEDIUM)
                    .color(palette::Text::DEFAULT),
                icon(Icon::lucide().chevron_down(), 12.0, palette::Text::MUTED, 0.68),
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
        .height(450)
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

    /// Everything inside the card. The trailing `Fill` space pins the footer
    /// to the card's bottom edge like the reference.
    fn card_content(&self) -> Element<'_, Message> {
        column![
            self.logo_row(),
            Space::new().height(22),
            self.headline(),
            Text::new("Sign in to sync your worlds, instances, and friends across devices.")
                .size(13)
                .font(fonts::Inter::REGULAR)
                .color(palette::Text::MUTED)
                .line_height(1.5),
            Space::new().height(22),
            self.sign_in_area(),
            Space::new().height(Fill),
            rule(),
            Space::new().height(12),
            self.footer(),
        ]
        .spacing(6)
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
                Text::new("Lodestone")
                    .size(22)
                    .font(fonts::Inter::EXTRA_BOLD)
                    .color(palette::Text::HEADLINE),
                mono(format!("THIRD-PARTY LAUNCHER · v{}", LodestoneLauncher::VERSION), 11.0),
            ]
            .spacing(2),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// "Welcome back, crafter" — Inter ExtraBold, per the reference.
    fn headline(&self) -> Element<'_, Message> {
        row![
            Text::new("Welcome back, ")
                .size(26)
                .font(fonts::Inter::EXTRA_BOLD)
                .color(palette::Text::HEADLINE),
            Text::new("crafter")
                .size(26)
                .font(fonts::Inter::EXTRA_BOLD)
                .color(palette::Primary::DEFAULT),
        ]
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// The Microsoft device-code area and its alternates, per flow state.
    /// The 18px rhythm matches the reference's collapsed margins.
    fn sign_in_area(&self) -> Element<'_, Message> {
        let mut body = column![].spacing(18);
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
            Flow::Code {
                user_code,
                verification_uri,
                copied,
            } => {
                body = body.push(self.code_panel(user_code, verification_uri, *copied));
                body = body.push(self.cancel_button());
            }
            Flow::Opening => {
                body = body.push(self.microsoft_button("Opening Microsoft…", Message::SignInMicrosoft).disabled(true));
            }
            // Idle and Failed both offer the button (retry after a failure).
            Flow::Idle | Flow::Failed(_) => {
                body = body.push(self.microsoft_button("Continue with Microsoft", Message::SignInMicrosoft));
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

    /// The device code on screen while the poll runs.
    /// The device code on screen while the poll runs — compact, so the
    /// fixed-height card never overflows. Clicking the code copies it;
    /// clicking the link opens the browser.
    fn code_panel(&self, user_code: &str, verification_uri: &str, copied: bool) -> Element<'_, Message> {
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

    /// Abandons an in-flight sign-in (the root invalidates the poll).
    fn cancel_button(&self) -> Element<'_, Message> {
        TextButton::content(
            row![
                Space::new().width(Fill),
                Text::new("Cancel").size(13).font(fonts::Inter::MEDIUM).color(palette::Text::DEFAULT),
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
            Text::new("OR").size(10).font(fonts::Inter::SEMI_BOLD).color(palette::Text::PLACEHOLDER),
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
                row![
                    Space::new().width(Fill),
                    icon(Icon::lucide().user(), 15.0, palette::Text::MUTED, 0.68),
                    Text::new("Offline Mode")
                        .size(13)
                        .font(fonts::Inter::MEDIUM)
                        .color(palette::Text::DEFAULT),
                    Space::new().width(Fill),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .width(Fill),
            )
            .width(Fill)
            .padding(Padding::new(0.0).top(12).bottom(12))
            .on_press(Message::OfflineMode),
            TextButton::content(
                row![
                    Space::new().width(Fill),
                    icon(Icon::lucide().shield(), 15.0, palette::Text::MUTED, 0.68),
                    Text::new("Demo Mode").size(13).font(fonts::Inter::MEDIUM).color(palette::Text::DEFAULT),
                    Space::new().width(Fill),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center)
                .width(Fill),
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
            Text::new("Not affiliated with Mojang or Microsoft")
                .size(11)
                .font(fonts::Inter::REGULAR)
                .color(palette::Text::PLACEHOLDER),
            Space::new().width(Fill),
            mouse_area(
                Text::new("Learn more ↗")
                    .size(11)
                    .font(fonts::Inter::MEDIUM)
                    .color(palette::Primary::DEFAULT)
            )
            .on_press(Message::LearnMore)
            .interaction(iced::mouse::Interaction::Pointer),
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

/// An SVG icon from a lucide markup string. The tint supplies the color
/// (its alpha is ignored by the svg texture pipeline) and `opacity`
/// controls visibility — 0.0 hides the icon while keeping its layout
/// space, so hover states never shift the layout.
fn icon(markup: &'static str, size: f32, tint: Color, opacity: f32) -> Element<'static, Message> {
    svg(Handle::from_memory(markup.as_bytes()))
        .width(size)
        .height(size)
        .opacity(opacity)
        .style(move |_, _| svg::Style { color: Some(tint) })
        .into()
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

/// A 1px underline for the verification link, exactly the width of its
/// fixed mono text ("microsoft.com/link" = 18 chars). Always occupies
/// space; visible only on hover.
fn rule_under_link(visible: bool) -> Element<'static, Message> {
    container(Space::new().height(1).width(119))
        .width(119)
        .style(move |_| container::Style {
            background: Some(Background::Color(if visible { palette::Text::HEADLINE } else { Color::TRANSPARENT })),
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
                verification_uri: "https://www.microsoft.com/link".into(),
                copied: false
            }
        );

        login.update(Message::CodeCopied);
        assert_eq!(
            login.flow,
            Flow::Code {
                user_code: "ABCD-EFGH".into(),
                verification_uri: "https://www.microsoft.com/link".into(),
                copied: true
            }
        );

        login.update(Message::SignInFailed("the sign-in request was denied".into()));
        assert_eq!(login.flow, Flow::Failed("the sign-in request was denied".into()));

        login.update(Message::Reset);
        assert_eq!(login.flow, Flow::Idle);
    }

    #[test]
    fn hover_flags_track_their_messages() {
        let mut login = Login::default();
        assert!(!login.code_hover && !login.link_hover);

        login.update(Message::CodeHover(true));
        login.update(Message::LinkHover(true));
        assert!(login.code_hover && login.link_hover);

        login.update(Message::CodeHover(false));
        assert!(!login.code_hover && login.link_hover);

        // Resetting the flow clears hover state too.
        login.update(Message::Reset);
        assert!(!login.code_hover && !login.link_hover);
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
