//! The root view: the login gate, or the titlebar, sidebar, and the routed page.

use crate::app::components::backdrop::LoginBackdrop;
use crate::app::components::sidebar::sidebar as sidebar_view;
use crate::app::pages::Page;
use crate::app::pages::login::AuthStatus;
use crate::app::theme::color::Surface;
use iced::widget::{Space, column, container, row, stack};
use iced::{Element, Fill};
use iced_borderless_window_resize_handle_widget::ResizeHandles;
use tracing::debug;

use super::{Account, LodestoneAppMessage, LodestoneLauncher, NavGroup};

impl LodestoneLauncher {
    pub fn view(&self) -> Element<'_, LodestoneAppMessage> {
        let content: Element<'_, LodestoneAppMessage> = if self.router.current() == Page::Login {
            // The login gate: the backdrop fills the whole window and the
            // chrome floats over it, transparent. No rail, no history.
            let auth_status = self.auth_status.as_ref().map(|result| match result {
                Ok(probe) => {
                    let region = match (&probe.region_scope, &probe.region_sub_scope) {
                        (Some(scope), Some(sub)) if !sub.is_empty() => Some(format!("{scope}-{sub}")),
                        (Some(scope), _) => Some(scope.clone()),
                        _ => None,
                    };
                    AuthStatus {
                        online: true,
                        delay_ms: probe.delay_ms,
                        region,
                    }
                }
                Err(_) => AuthStatus {
                    online: false,
                    delay_ms: 0,
                    region: None,
                },
            });
            stack![
                LoginBackdrop::new(),
                column![
                    self.titlebar
                        .view(self.router.can_go_back(), self.router.can_go_forward(), true)
                        .map(LodestoneAppMessage::TitlebarMessage),
                    container(self.login.view(auth_status).map(LodestoneAppMessage::Login))
                        .width(Fill)
                        .height(Fill),
                ]
                .width(Fill)
                .height(Fill),
            ]
            .into()
        } else {
            column![
                self.titlebar
                    .view(self.router.can_go_back(), self.router.can_go_forward(), false)
                    .map(LodestoneAppMessage::TitlebarMessage),
                row![self.rail_column(), self.body()].width(Fill).height(Fill),
            ]
            .width(Fill)
            .height(Fill)
            .into()
        };
        // An open alert covers the titlebar and sidebar too; the resize
        // handles stay above it so the window can still be resized. The slot
        // is always present so the layers keep their tree positions.
        let alert: Element<'_, LodestoneAppMessage> = match self.alerts.view() {
            Some(alert) => alert.map(LodestoneAppMessage::Alert),
            None => Space::new().into(),
        };
        // The New Instance dialog covers the page the way an alert does, under alerts.
        let dialog: Element<'_, LodestoneAppMessage> = match self.library.modal() {
            Some(dialog) => dialog.map(LodestoneAppMessage::Library),
            None => Space::new().into(),
        };
        stack![
            content,
            dialog,
            alert,
            ResizeHandles::new(LodestoneAppMessage::WindowStartResize)
                .gutter(4.0)
                .enabled(!self.maximized)
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }

    /// The full sidebar — logo, all nav groups, and the player avatar. The
    /// top group's first item (Library) navigates; the rest are display-only
    /// until their pages exist.
    fn rail_column(&self) -> Element<'_, LodestoneAppMessage> {
        sidebar_view(
            self.face.clone(),
            self.account.as_ref().map(Account::username).map(str::to_string),
            matches!(self.account, Some(Account::Microsoft { .. })),
            0,
            |index| {
                if index == 0 {
                    LodestoneAppMessage::Navigate(Page::Home)
                } else {
                    debug!("Sidebar nav item {index} has no page yet");
                    LodestoneAppMessage::SidebarNav { group: NavGroup::Top, index }
                }
            },
            |index| LodestoneAppMessage::SidebarNav {
                group: NavGroup::Bottom,
                index,
            },
        )
    }

    /// Left edge chrome — one rail slot per [`Page::RAIL_PAGES`] entry.
    /// The page area: the router's current page.
    fn body(&self) -> Element<'_, LodestoneAppMessage> {
        container(self.outlet())
            .width(Fill)
            .height(Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(Surface::APPLICATION)),
                ..Default::default()
            })
            .into()
    }

    /// The `<Outlet/>` — whichever page the router points at. The login gate
    /// renders through `view`'s dedicated branch, so this arm only serves
    /// callers that ask for the current page generically.
    fn outlet(&self) -> Element<'_, LodestoneAppMessage> {
        match self.router.current() {
            Page::Login => self.login.view(None).map(LodestoneAppMessage::Login),
            Page::Home => self.library.view(self.window_size).map(LodestoneAppMessage::Library),
            Page::Instance => self.instance_page.view(self.window_size).map(LodestoneAppMessage::Instance),
            Page::Settings => self.settings.view().map(LodestoneAppMessage::Settings),
            Page::Showcase => self.showcase.view().map(LodestoneAppMessage::Showcase),
        }
    }
}
