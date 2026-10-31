//! Ongoing root subscriptions: window resizes, mouse history buttons, the
//! auth-services probe timer, the instance page's log refresh, and the F3
//! performance overlay.

use iced::window::Id;
use iced::{Subscription, event, mouse, window};

use super::pages::Page;
use super::{LodestoneAppMessage, LodestoneLauncher, perf};

impl LodestoneLauncher {
    pub fn subscription(&self) -> Subscription<LodestoneAppMessage> {
        Subscription::batch([
            window::resize_events().map(|(id, size)| LodestoneAppMessage::WindowSizeChanged(id, size)),
            event::listen_with(Self::mouse_navigation),
            // The auth-services probe — keeps the login status bar honest.
            iced::time::every(std::time::Duration::from_secs(15)).map(|_| LodestoneAppMessage::AuthProbeTick),
            self.instance_subscription(),
            perf::subscription(&self.perf).map(LodestoneAppMessage::Perf),
        ])
    }

    /// The instance page's log refresh. It runs only while that page is on screen.
    fn instance_subscription(&self) -> Subscription<LodestoneAppMessage> {
        match self.router.current() {
            Page::Instance => self.instance_page.subscription(self.alerts.is_open()).map(LodestoneAppMessage::Instance),
            _ => Subscription::none(),
        }
    }

    /// Maps the mouse's side buttons onto history navigation — back and
    /// forward, like a browser.
    ///
    /// Only uncaptured presses count, so widgets that consume an event keep
    /// it to themselves. Delivery depends on the platform exposing the
    /// hardware buttons through the windowing system (standard on Windows,
    /// X11 buttons 8/9 on Linux, compositor-dependent on Wayland). The app
    /// is single-window, so the source window is irrelevant.
    fn mouse_navigation(event: iced::Event, status: event::Status, _window: Id) -> Option<LodestoneAppMessage> {
        if status != event::Status::Ignored {
            return None;
        }
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Back)) => Some(LodestoneAppMessage::NavBack),
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Forward)) => Some(LodestoneAppMessage::NavForward),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/app/subscription.rs"]
mod tests;
