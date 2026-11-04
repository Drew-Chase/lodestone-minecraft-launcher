//! The root view: the login gate, or the titlebar, sidebar, and the routed page.

use crate::app::components::backdrop::LoginBackdrop;
use crate::app::components::game_controls::Actions;
use crate::app::components::hint;
use crate::app::components::page::{MAX_CONTENT_WIDTH, content_width};
use crate::app::components::sidebar::{game_block, sidebar as sidebar_view};
use crate::app::coop_sync;
use crate::app::data_folder;
use crate::app::downloads::{self, Pending};
use crate::app::lang;
use crate::app::launch::GameMessage;
use crate::app::launch::games::{GameBar, RunsPlace};
use crate::app::pages::Page;
use crate::app::pages::friends;
use crate::app::pages::login::AuthStatus;
use crate::app::perf;
use crate::app::servers;
use crate::app::theme::color;
use iced::widget::tooltip::Position;
use iced::widget::{Space, column, container, row, stack};
use iced::{Element, Fill, alignment};
use iced_borderless_window_resize_handle_widget::ResizeHandles;
use tracing::debug;

use super::{Account, LodestoneAppMessage, LodestoneLauncher, NavGroup};

/// The pages the top nav group opens, in the order the rail lists them: Library, Discover, Worlds, Skins, Servers and
/// Friends.
const TOP_NAV: [Page; 6] = [Page::Library, Page::Discover, Page::Worlds, Page::Skins, Page::Servers, Page::Friends];

/// The top nav item a page highlights. Discover and its resource pages share one, and a page outside the group highlights
/// Library.
fn top_nav_index(page: Page) -> usize {
    match page {
        Page::Discover | Page::Resource => 1,
        page => TOP_NAV.iter().position(|item| *item == page).unwrap_or(0),
    }
}

impl LodestoneLauncher {
    pub fn view(&self) -> Element<'_, LodestoneAppMessage> {
        let _scope = perf::scope("view.root");
        let content: Element<'_, LodestoneAppMessage> = if self.router.current() == Page::Login || self.portable_signing_in() {
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
                        .view(self.router.can_go_back(), self.router.can_go_forward(), true, &GameBar::default(), false)
                        .map(LodestoneAppMessage::TitlebarMessage),
                    container(self.login.view(auth_status).map(LodestoneAppMessage::Login))
                        .width(Fill)
                        .height(Fill),
                ]
                .width(Fill)
                .height(Fill),
            ]
            .into()
        } else if self.portable.is_some() {
            // A portable launcher has no library: its page sits under the same chrome as the sign-in gate.
            stack![
                LoginBackdrop::new(),
                column![
                    self.titlebar
                        .view(self.router.can_go_back(), self.router.can_go_forward(), true, &GameBar::default(), false)
                        .map(LodestoneAppMessage::TitlebarMessage),
                    self.portable_page(),
                ]
                .width(Fill)
                .height(Fill),
            ]
            .into()
        } else {
            let bar = self.game_bar();
            column![
                self.titlebar
                    .view(
                        self.router.can_go_back(),
                        self.router.can_go_forward(),
                        false,
                        &bar,
                        self.runs_open.is_open(RunsPlace::Titlebar),
                    )
                    .map(LodestoneAppMessage::TitlebarMessage),
                row![self.rail_column(&bar), self.body()].width(Fill).height(Fill),
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
        // The export dialog covers the page the same way, under the alerts.
        let export: Element<'_, LodestoneAppMessage> = match self.export.as_ref() {
            Some(dialog) => dialog.view().map(LodestoneAppMessage::Export),
            None => Space::new().into(),
        };
        // The import dialog covers the page the same way, above the export dialog.
        let import: Element<'_, LodestoneAppMessage> = match self.import.as_ref() {
            Some(dialog) => dialog.view().map(LodestoneAppMessage::Import),
            None => Space::new().into(),
        };
        // The Servers dialogs cover the page the same way, above the import dialog. The Join Server dialog sits under the Add
        // server one it can open.
        let join: Element<'_, LodestoneAppMessage> = match self.join_dialog.as_ref() {
            Some(dialog) => self.join_dialog_view(dialog).map(LodestoneAppMessage::JoinDialog),
            None => Space::new().into(),
        };
        let servers_dialog: Element<'_, LodestoneAppMessage> = match self.servers_dialog.as_ref() {
            Some(dialog) => servers::dialog::view(dialog).map(LodestoneAppMessage::ServersDialog),
            None => Space::new().into(),
        };
        // The Co-op Sync dialog covers the page the same way, above the Servers dialogs.
        let coop: Element<'_, LodestoneAppMessage> = if self.coop_sync_open {
            coop_sync::view().map(LodestoneAppMessage::CoopSync)
        } else {
            Space::new().into()
        };
        // The changelog of a newer launcher release covers the page the same way, above the Co-op Sync dialog.
        let changelog: Element<'_, LodestoneAppMessage> = match self.updater.modal() {
            Some(modal) => modal.map(LodestoneAppMessage::Updater),
            None => Space::new().into(),
        };
        // The screenshot viewer covers the whole window, under the alerts. It is open only on the instance page.
        let viewer: Element<'_, LodestoneAppMessage> = match self.router.current() {
            Page::Instance => match self.instance_page.viewer(self.window_size) {
                Some(viewer) => viewer.map(LodestoneAppMessage::Instance),
                None => Space::new().into(),
            },
            // The gallery's picture viewer covers the resource page the same way.
            Page::Resource => match self.resource.viewer() {
                Some(viewer) => viewer.map(LodestoneAppMessage::Resource),
                None => Space::new().into(),
            },
            _ => Space::new().into(),
        };
        // The cover for a data move sits above every dialog, so nothing runs on the folder while it moves.
        let moving: Element<'_, LodestoneAppMessage> = match self.data_move.as_deref() {
            Some(to) => data_folder::moving_cover(to),
            None => Space::new().into(),
        };
        // The overlay sits under the resize handles and above everything else. It is an empty slot unless
        // the `perf` feature shows it, so the layers keep their tree positions either way.
        let hud: Element<'_, LodestoneAppMessage> = perf::overlay(&self.perf);
        perf::frame(
            stack![
                content,
                dialog,
                export,
                import,
                join,
                servers_dialog,
                coop,
                changelog,
                viewer,
                alert,
                moving,
                hud,
                ResizeHandles::new(LodestoneAppMessage::WindowStartResize)
                    .gutter(4.0)
                    .enabled(!self.maximized)
            ]
            .width(Fill)
            .height(Fill)
            .into(),
        )
    }

    /// The install of the instance the page shows, while that install has not ended.
    fn instance_install(&self) -> Option<&Pending> {
        let instance = self.instance_page.instance()?;
        self.library.pending_install(&instance.id)
    }

    /// The full sidebar — logo, all nav groups, the game block, and the player avatar. The
    /// top group's first item (Library) navigates; the rest are display-only
    /// until their pages exist.
    fn rail_column(&self, bar: &GameBar) -> Element<'_, LodestoneAppMessage> {
        // The game block and the Downloads trigger sit above the bottom group, in that order.
        let above_bottom = column![
            game_block(
                bar,
                self.runs_open.is_open(RunsPlace::Sidebar),
                &Actions {
                    toggle: LodestoneAppMessage::Game(GameMessage::ToggleRuns(RunsPlace::Sidebar)),
                    dismiss: LodestoneAppMessage::Game(GameMessage::DismissRuns(RunsPlace::Sidebar)),
                    kill: |serial: u64| LodestoneAppMessage::Game(GameMessage::KillRun(serial)),
                    console: |serial: u64| LodestoneAppMessage::Game(GameMessage::ConsoleRun(serial)),
                    launch: |id: String| LodestoneAppMessage::Game(GameMessage::LaunchNew(id)),
                },
            ),
            hint::at(
                downloads::trigger(&self.downloads).map(LodestoneAppMessage::Downloads),
                lang::t("nav.downloads"),
                Position::Right,
            ),
        ]
        .spacing(6);
        sidebar_view(
            self.face.clone(),
            self.account.as_ref().map(Account::username).map(str::to_string),
            matches!(self.account, Some(Account::Microsoft { .. })),
            top_nav_index(self.router.current()),
            |index| match TOP_NAV.get(index) {
                Some(page) => LodestoneAppMessage::Navigate(*page),
                None => {
                    debug!("Sidebar nav item {index} has no page yet");
                    LodestoneAppMessage::SidebarNav { group: NavGroup::Top, index }
                }
            },
            |index| LodestoneAppMessage::SidebarNav {
                group: NavGroup::Bottom,
                index,
            },
            LodestoneAppMessage::Navigate(Page::News),
            above_bottom.into(),
        )
    }

    /// The page area: the router's current page. The page sits in a column at most [`MAX_CONTENT_WIDTH`]
    /// wide, centred beside the sidebar, so a wide window keeps the page at its designed width.
    fn body(&self) -> Element<'_, LodestoneAppMessage> {
        let page = container(self.outlet()).max_width(MAX_CONTENT_WIDTH).width(Fill).height(Fill);
        container(page)
            .width(Fill)
            .height(Fill)
            .align_x(alignment::Horizontal::Center)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(color::surface_application())),
                ..Default::default()
            })
            .into()
    }

    /// The `<Outlet/>` — whichever page the router points at. The login gate
    /// renders through `view`'s dedicated branch, so this arm only serves
    /// callers that ask for the current page generically.
    fn outlet(&self) -> Element<'_, LodestoneAppMessage> {
        let page = self.router.current();
        let _scope = perf::scope(match page {
            Page::Login => "view.login",
            Page::Library => "view.library",
            Page::News => "view.news",
            Page::Instance => "view.instance",
            Page::Resource => "view.resource",
            Page::Settings => "view.settings",
            Page::Discover => "view.discover",
            Page::Worlds => "view.worlds",
            Page::Skins => "view.skins",
            Page::Servers => "view.servers",
            Page::Friends => "view.friends",
        });
        match page {
            Page::Login => self.login.view(None).map(LodestoneAppMessage::Login),
            Page::Library => self
                .library
                .view(self.window_size, &self.art, self.updater.button(), &self.notifications)
                .map(LodestoneAppMessage::Library),
            Page::News => self.news.view(self.window_size).map(LodestoneAppMessage::News),
            // The Servers and Friends pages are filled in by their own phases.
            Page::Servers => self.servers.view().map(LodestoneAppMessage::Servers),
            Page::Friends => friends::view(),
            Page::Instance => self
                .instance_page
                .view(self.window_size, self.page_game_view(), self.instance_install(), &self.world_pins)
                .map(LodestoneAppMessage::Instance),
            Page::Resource => match self.resource_status() {
                Some(status) => self.resource.view(self.window_size, &status).map(LodestoneAppMessage::Resource),
                None => Space::new().into(),
            },
            Page::Settings => self
                .settings
                .view(
                    self.auth_status.as_ref().map(Result::is_ok),
                    &self.speed,
                    &self.credentials,
                    self.face.as_ref(),
                )
                .map(LodestoneAppMessage::Settings),
            Page::Discover => self.discover.view(self.window_size).map(LodestoneAppMessage::Discover),
            Page::Worlds => self.worlds.view(self.window_size).map(LodestoneAppMessage::Worlds),
            Page::Skins => self
                .skins
                .view(self.skins_account(), content_width(self.window_size.width))
                .map(LodestoneAppMessage::Skins),
        }
    }
}
