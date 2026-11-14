//! Ongoing root subscriptions: window resizes, mouse history buttons, the
//! auth-services probe timer, the instance page's log refresh, and the F3
//! performance overlay.

use iced::window::Id;
use iced::{Subscription, event, keyboard, mouse, window};

use super::games;
use super::import;
use super::launch::GameMessage;
use super::notifications;
use super::pages::Page;
use super::pages::skins::{Message as SkinsMessage, PREVIEW_FRAME};
use super::retention;
use super::servers;
use super::updater;
use super::{LodestoneAppMessage, LodestoneLauncher, perf};

impl LodestoneLauncher {
    pub fn subscription(&self) -> Subscription<LodestoneAppMessage> {
        Subscription::batch([
            window::resize_events().map(|(id, size)| LodestoneAppMessage::WindowSizeChanged(id, size)),
            event::listen_with(Self::mouse_navigation),
            event::listen_with(Self::window_scale),
            // Pictures and caches that nobody has used for a while are dropped on each tick. See `retention`.
            iced::time::every(retention::SWEEP_EVERY).map(|_| LodestoneAppMessage::SweepCaches),
            self.import_subscription(),
            self.auth_probe_subscription(),
            self.speed_subscription(),
            self.instance_subscription(),
            self.skins_subscription(),
            self.discover_shortcut(),
            self.servers_subscription(),
            // The update checks are due every few hours, so the clock wakes once an hour to see whether one is.
            iced::time::every(std::time::Duration::from_secs(60 * 60)).map(|_| LodestoneAppMessage::Notifications(notifications::Message::CheckDue)),
            // The launcher checks GitHub for a newer release every half hour while it runs.
            iced::time::every(updater::CHECK_EVERY).map(|_| LodestoneAppMessage::Updater(updater::Message::CheckDue)),
            perf::subscription(&self.perf).map(LodestoneAppMessage::Perf),
            super::single_instance::focus_requests().map(|()| LodestoneAppMessage::FocusRequested),
            // A close is asked for here, so a running game can be asked about before the launcher goes.
            window::close_requests().map(LodestoneAppMessage::CloseRequested),
            self.playtime_subscription(),
        ])
    }

    /// The playtime checkpoint: each minute while a game runs, so the time it has run is saved as it goes, not only when it
    /// exits.
    fn playtime_subscription(&self) -> Subscription<LodestoneAppMessage> {
        if self.games.any_running() {
            iced::time::every(games::CHECKPOINT_EVERY).map(|_| LodestoneAppMessage::Game(GameMessage::Checkpoint))
        } else {
            Subscription::none()
        }
    }

    /// The auth-services probe runs while a page shows its result: the sign-in page's status bar and the Settings page. Elsewhere
    /// nothing reads it, so it does not wake the window.
    fn auth_probe_subscription(&self) -> Subscription<LodestoneAppMessage> {
        match self.router.current() {
            Page::Login | Page::Settings => iced::time::every(std::time::Duration::from_secs(15)).map(|_| LodestoneAppMessage::AuthProbeTick),
            _ => Subscription::none(),
        }
    }

    /// The speed meter samples once a second while the Settings page shows its chart, or while bytes are moving: a download
    /// or a launch that is preparing. Each sample redraws the window, so an idle launcher does not take them.
    fn speed_subscription(&self) -> Subscription<LodestoneAppMessage> {
        let moving = self.downloads.running() > 0 || self.games.any_preparing();
        if matches!(self.router.current(), Page::Settings) || moving {
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| LodestoneAppMessage::SpeedTick)
        } else {
            Subscription::none()
        }
    }

    /// The Servers page's clocks: its round of pings every few seconds, and the games announced on the local network. They
    /// run only while the page is on screen.
    fn servers_subscription(&self) -> Subscription<LodestoneAppMessage> {
        match self.router.current() {
            Page::Servers => servers::subscription(),
            _ => Subscription::none(),
        }
    }

    /// The file events the Import dialog listens for: a file dragged over the window, leaving it, or dropped on it. They are
    /// listened for only while the dialog is open.
    fn import_subscription(&self) -> Subscription<LodestoneAppMessage> {
        match self.import {
            Some(_) => event::listen_with(import::drop_event),
            None => Subscription::none(),
        }
    }

    /// Ctrl/⌘K focuses the Discover search field. It is listened for only while the page is on
    /// screen, so the keys mean nothing elsewhere.
    fn discover_shortcut(&self) -> Subscription<LodestoneAppMessage> {
        match self.router.current() {
            Page::Discover => event::listen_with(Self::discover_key),
            _ => Subscription::none(),
        }
    }

    /// Ctrl/⌘K, whether or not a widget has the key: the search field takes the focus either way.
    fn discover_key(event: iced::Event, _status: event::Status, _window: Id) -> Option<LodestoneAppMessage> {
        match event {
            iced::Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                let is_k = matches!(key.as_ref(), keyboard::Key::Character("k") | keyboard::Key::Character("K"));
                (is_k && modifiers.command()).then_some(LodestoneAppMessage::Discover(super::pages::discover::Message::FocusSearch))
            }
            _ => None,
        }
    }

    /// The Skins page's preview clock: its previews take a step through the poses at each frame. It runs only while that page is on
    /// screen, and not under reduced motion, where the poses are held.
    fn skins_subscription(&self) -> Subscription<LodestoneAppMessage> {
        match self.router.current() {
            Page::Skins if super::theme::effects::ambient() => {
                iced::time::every(PREVIEW_FRAME).map(|now| LodestoneAppMessage::Skins(SkinsMessage::Tick(now)))
            }
            _ => Subscription::none(),
        }
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

    /// The window moved to a screen of another scale, so the pictures sized to it are sized again from its new factor.
    fn window_scale(event: iced::Event, _status: event::Status, _window: Id) -> Option<LodestoneAppMessage> {
        match event {
            iced::Event::Window(window::Event::Rescaled(factor)) => Some(LodestoneAppMessage::ScaleFactor(factor)),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "../../tests/app/subscription.rs"]
mod tests;
