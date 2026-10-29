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

mod chrome;
mod panels;
mod sign_in;
mod widgets;

use iced::widget::{column, container};
use iced::{Element, Fill, Padding};
use lodestone_minecraft_authentication::{DeviceCodeStart, Session};

use crate::app::pages::Page;

pub(crate) use widgets::logo_handle;

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
    /// Offline mode picked — the user must enter a username.
    OfflinePrompt {
        /// The username as typed (pre-filled from storage).
        username: String,
    },
    /// A stored Microsoft sign-in is being restored silently.
    Restoring { username: String },
    /// The last attempt failed with a user-presentable message; the card
    /// offers a retry.
    Failed(String),
}

/// The auth-services status shown in the status bar — the login page's
/// view of the root's probe results.
#[derive(Debug, Clone)]
pub struct AuthStatus {
    /// The services answered the probe.
    pub online: bool,
    /// Measured round-trip of the probe, in milliseconds (0 when the
    /// services did not answer).
    pub delay_ms: u64,
    /// The region scope Microsoft reported for this address, when it
    /// reported one.
    pub region: Option<String>,
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
    /// "Offline Mode" picked — the root opens the username prompt,
    /// pre-filled with the stored offline username when there is one.
    OfflinePrompt { username: String },
    /// The offline username changed as it was typed.
    OfflineUsernameChanged(String),
    /// The offline username form was submitted (button or Enter key).
    OfflineConfirm { username: String },
    /// A stored Microsoft sign-in is being restored — display state only.
    Restoring { username: String },
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
            Message::OfflinePrompt { username } => {
                self.flow = Flow::OfflinePrompt { username };
                self.code_hover = false;
                self.link_hover = false;
            }
            Message::OfflineUsernameChanged(username) => {
                if let Flow::OfflinePrompt { username: stored, .. } = &mut self.flow {
                    *stored = username;
                }
            }
            Message::Restoring { username } => self.flow = Flow::Restoring { username },
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
            | Message::OfflineConfirm { .. }
            | Message::DeviceCodeStarted(_, _)
            | Message::SignInFinished(_, _) => {}
        }
    }

    pub fn view(&self, auth_status: Option<AuthStatus>) -> Element<'_, Message> {
        column![
            // Top chrome overlays: language chip left, brand centered.
            container(self.top_bar())
                .width(Fill)
                .padding(Padding::new(0.0).left(24).right(24).top(12)),
            // The card, centered in the remaining space.
            container(self.card()).width(Fill).height(Fill).center(Fill),
            self.status_bar(auth_status),
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/login/mod.rs"]
mod tests;
