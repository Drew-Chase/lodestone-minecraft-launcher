//! The Servers page: the servers the player saved and the servers each instance's `servers.dat` lists, each with its live ping,
//! the games announced on the local network, and the buttons that join, save, edit and remove them. Pings run while the page
//! is open, every ten seconds.

mod header;
mod listed;
mod row;
mod scan;

pub use scan::{InstanceServers, read_all};

use std::collections::HashMap;
use std::time::{Duration, Instant};

use iced::widget::image;
use iced::widget::{Column, Space, column, container};
use iced::{Element, Fill, Padding, Size, alignment};
use lodestone_server::address::ServerAddress;
use lodestone_server::lan::Announcement;
use lodestone_server::status::Status;

use crate::app::components::Tabs;
use crate::app::components::badge::Badge;
use crate::app::components::scrollbar;
use crate::app::database::servers::SavedServer;
use crate::app::lang;
use crate::app::theme::{color, fonts};
use listed::{Listed, merge};

/// How long a game stays listed after its last announcement, before it counts as gone.
const LAN_KEPT: Duration = Duration::from_secs(15);

/// Which list the page shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// Every saved server.
    #[default]
    All,
    /// The saved servers marked as favorites.
    Favorites,
    /// The Realms the player belongs to. Realms are coming soon, so the tab says so.
    Realms,
    /// The games announced on the local network.
    Lan,
}

impl Tab {
    /// The tabs in the order the page lists them.
    pub const ALL: [Tab; 4] = [Tab::All, Tab::Favorites, Tab::Realms, Tab::Lan];
}

/// What a server answered with its last ping: its status, and its icon when it has one.
#[derive(Debug, Clone)]
pub struct Reply {
    pub status: Status,
    pub icon: Option<image::Handle>,
}

/// A saved server's ping: whether it has answered yet, and what it said or why it did not.
#[derive(Debug, Clone)]
pub enum Probe {
    /// The ping has not come back yet.
    Waiting,
    /// The server answered.
    Online(Box<Reply>),
    /// The server did not answer, and the reason.
    Offline(String),
}

/// What a row's menu offers, in the order the menu lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowAction {
    Join,
    Copy,
    Favorite,
    Edit,
    Remove,
}

impl RowAction {
    /// The actions a saved server's menu lists, in order. A menu row is picked by its place in this list.
    pub const MENU: [RowAction; 5] = [RowAction::Join, RowAction::Copy, RowAction::Favorite, RowAction::Edit, RowAction::Remove];
    /// The actions a server the launcher has not saved lists. Its favorite mark saves it to the launcher's list.
    pub const UNSAVED_MENU: [RowAction; 3] = [RowAction::Join, RowAction::Copy, RowAction::Favorite];

    /// The actions a row's menu lists, for a server the launcher `saved` or has not.
    pub fn menu(saved: bool) -> &'static [RowAction] {
        if saved { &Self::MENU } else { &Self::UNSAVED_MENU }
    }
}

/// The Servers page's state: the saved servers, what the instances list, their pings, the games heard on the network, and what
/// the page shows.
#[derive(Default)]
pub struct State {
    saved: Vec<SavedServer>,
    /// What each instance's `servers.dat` lists, in the order the instances come.
    instances: Vec<InstanceServers>,
    /// The saved servers, the instances' lists and the games announced now, merged by address: the favorites first, then by name.
    listed: Vec<Listed>,
    /// The latest ping of each server, by the address as it is written.
    probes: HashMap<String, Probe>,
    /// The games heard on the local network, each with when it was last announced.
    lan: Vec<(Instant, Announcement)>,
    tab: Tab,
    query: String,
}

/// What the page sends, and what the root sends back when the work it started finishes.
#[derive(Debug, Clone)]
pub enum Message {
    /// The saved servers were read from the database, or the read failed.
    Loaded(Result<Vec<SavedServer>, String>),
    /// A round of pings is due: the page opened, the ten-second clock ticked, or Refresh was pressed.
    Round,
    /// A server's ping came back, or did not.
    Probed { address: String, result: Result<Reply, String> },
    /// A game was announced on the local network.
    Heard(Announcement),
    /// A tab was picked.
    Tab(Tab),
    /// The search field changed.
    Query(String),
    /// Enter was pressed in the search field.
    Submit,
    /// The Add server button was pressed.
    Add,
    /// A row's menu was picked: the row's address, and the item's place in [`RowAction::MENU`].
    RowMenu { address: String, index: usize },
    /// A row's Join button was pressed, by the address it joins.
    Join(String),
    /// A row's favorite star was pressed, by the saved server's id.
    ToggleFavorite(i64),
    /// A server was saved to the launcher's list, by its name, address and favorite mark. A game on the local network is saved
    /// without the mark, and a server an instance lists is saved as a favorite when its star is pressed.
    Save { name: String, address: String, favorite: bool },
    /// Every instance's `servers.dat` was read: one list for each instance that holds servers.
    InstanceLists(Vec<InstanceServers>),
    /// A saved change was written, or the write failed. The list is read again.
    Written(Result<(), String>),
    /// The question about removing a saved server was answered.
    RemoveAnswered { id: i64, confirmed: bool },
}

impl State {
    /// Takes the saved servers the database read, and merges them with the instances' lists.
    pub fn set_saved(&mut self, saved: Vec<SavedServer>) {
        self.saved = saved;
        self.rebuild();
    }

    /// Takes the servers each instance's `servers.dat` lists, and merges them with the saved servers.
    pub fn set_instances(&mut self, instances: Vec<InstanceServers>) {
        self.instances = instances;
        self.rebuild();
    }

    /// Merges the saved servers, the instances' lists and the games announced now, again, after any of them changed. A server
    /// that is no longer listed loses its ping with it.
    fn rebuild(&mut self) {
        let games: Vec<Announcement> = self.lan.iter().map(|(_, game)| game.clone()).collect();
        self.listed = merge(&self.saved, &self.instances, &games);
        let listed = &self.listed;
        self.probes.retain(|address, _| listed.iter().any(|server| &server.address == address));
    }

    /// The search text the page filters its lists by.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// The servers the launcher saved, favorites first. The database keeps each address once.
    pub fn saved(&self) -> &[SavedServer] {
        &self.saved
    }

    /// Every server the page lists: the saved ones and the ones an instance lists, each address once.
    pub fn listed(&self) -> &[Listed] {
        &self.listed
    }

    /// The listed server at `address`.
    pub fn listed_at(&self, address: &str) -> Option<&Listed> {
        self.listed.iter().find(|server| server.address == address)
    }

    /// Every listed server's address, as it is written, for a round of pings.
    pub fn addresses(&self) -> Vec<String> {
        self.listed.iter().map(|server| server.address.clone()).collect()
    }

    /// The saved server with `id`.
    pub fn server(&self, id: i64) -> Option<&SavedServer> {
        self.saved.iter().find(|server| server.id == id)
    }

    /// Marks a server as pinging. A server keeps its last answer while it waits for the next one.
    pub fn begin_probe(&mut self, address: &str) {
        self.probes.entry(address.to_owned()).or_insert(Probe::Waiting);
    }

    /// Keeps what a server answered with its ping, or why it did not answer.
    pub fn probed(&mut self, address: String, result: Result<Reply, String>) {
        let probe = match result {
            Ok(reply) => Probe::Online(Box::new(reply)),
            Err(error) => Probe::Offline(error),
        };
        self.probes.insert(address, probe);
    }

    /// The latest ping of the server at `address`.
    pub fn probe(&self, address: &str) -> Option<&Probe> {
        self.probes.get(address)
    }

    /// Keeps a game heard on the network, once per address, and drops the games not announced for a while.
    pub fn heard(&mut self, announcement: Announcement, now: Instant) {
        self.lan
            .retain(|(seen, known)| known.address != announcement.address && now.duration_since(*seen) < LAN_KEPT);
        self.lan.push((now, announcement));
        self.rebuild();
    }

    /// Drops the games not announced for a while. Called on each round, so a game that stopped is listed no longer.
    pub fn forget_quiet_games(&mut self, now: Instant) {
        self.lan.retain(|(seen, _)| now.duration_since(*seen) < LAN_KEPT);
        self.rebuild();
    }

    /// The games heard on the local network, newest first.
    pub fn lan(&self) -> Vec<&Announcement> {
        let mut games: Vec<&(Instant, Announcement)> = self.lan.iter().collect();
        games.sort_by_key(|(seen, _)| std::cmp::Reverse(*seen));
        games.into_iter().map(|(_, announcement)| announcement).collect()
    }

    /// Whether the search keeps the server or game named `text`, by its name or its address.
    fn matches(&self, name: &str, address: &str) -> bool {
        let query = self.query.trim().to_lowercase();
        query.is_empty() || name.to_lowercase().contains(&query) || address.to_lowercase().contains(&query)
    }

    /// The listed servers the tab and the search keep, in the order the page lists them.
    fn shown_servers(&self) -> Vec<&Listed> {
        self.listed
            .iter()
            .filter(|server| self.tab != Tab::Favorites || server.favorite())
            .filter(|server| self.matches(&server.name, &server.address))
            .collect()
    }

    /// How many servers answered, and how many players they hold between them.
    pub fn online_summary(&self) -> (usize, u32) {
        self.listed
            .iter()
            .filter_map(|server| self.probes.get(&server.address))
            .fold((0, 0), |(servers, players), probe| match probe {
                Probe::Online(reply) => (servers + 1, players + reply.status.online),
                _ => (servers, players),
            })
    }

    /// Applies a message the page answers itself. The messages the root answers are left alone.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Tab(tab) => self.tab = tab,
            Message::Query(query) => self.query = query,
            _ => {}
        }
    }

    /// The page: its header and tabs, and the list the tab shows beneath them.
    pub fn view(&self) -> Element<'_, Message> {
        let body: Element<'_, Message> = match self.tab {
            Tab::All | Tab::Favorites => self.servers_list(),
            Tab::Realms => self.realms_note(),
            Tab::Lan => self.lan_list(),
        };
        let live = Badge::new(lang::t("servers.live"), color::primary(), color::primary_tint()).with_pulse(true);
        let tabs = iced::widget::row![self.tabs(), Space::new().width(Fill), live].align_y(alignment::Vertical::Center);
        let page = column![header::view(self), tabs]
            .spacing(18)
            .push(scrollbar::scroll(body).width(Fill).height(Fill));
        container(page)
            .padding(Padding::new(0.0).top(24).left(28).right(28).bottom(0))
            .width(Fill)
            .height(Fill)
            .into()
    }

    /// The tabs under the header, each with its count, underlined, and the live badge at their right.
    fn tabs(&self) -> Element<'_, Message> {
        let labels = [
            format!("{} {}", lang::t("servers.tab.all"), self.listed.len()),
            format!(
                "{} {}",
                lang::t("servers.tab.favorites"),
                self.listed.iter().filter(|server| server.favorite()).count()
            ),
            format!("{} 0", lang::t("servers.tab.realms")),
            format!("{} {}", lang::t("servers.tab.lan"), self.lan.len()),
        ];
        let active = Tab::ALL.iter().position(|tab| *tab == self.tab).unwrap_or_default();
        Tabs::new(labels)
            .active(active)
            .on_select(|index| Message::Tab(Tab::ALL[index]))
            .underline()
            .into()
    }

    /// The servers the tab keeps, one row each, or a line saying there are none.
    fn servers_list(&self) -> Element<'_, Message> {
        let servers = self.shown_servers();
        if servers.is_empty() {
            let key = if self.listed.is_empty() {
                "servers.empty"
            } else if self.tab == Tab::Favorites {
                "servers.empty_favorites"
            } else {
                "servers.empty_search"
            };
            return empty_line(lang::t(key));
        }
        let rows = servers
            .into_iter()
            .map(|server| row::listed(server, self.probe(&server.address)))
            .collect::<Vec<_>>();
        Column::with_children(rows).spacing(12).width(Fill).into()
    }

    /// The Realms tab's note: Realms are coming soon.
    fn realms_note(&self) -> Element<'_, Message> {
        empty_line(lang::t("servers.realms.coming_soon"))
    }

    /// The games heard on the local network, one row each, or a line saying none has been announced yet.
    fn lan_list(&self) -> Element<'_, Message> {
        let games = self.lan();
        if games.is_empty() {
            return empty_line(lang::t("servers.lan.empty"));
        }
        let rows = games
            .into_iter()
            .filter(|game| self.matches(&game.title, &game.address.to_string()))
            .map(|game| row::heard(game))
            .collect::<Vec<_>>();
        Column::with_children(rows).spacing(12).width(Fill).into()
    }
}

/// A line of muted text where a list would be, for a list with nothing in it.
fn empty_line(text: &str) -> Element<'_, Message> {
    container(iced::widget::text(text).size(13).font(fonts::Inter::REGULAR).color(color::text_muted()))
        .padding(Padding::new(16.0))
        .width(Fill)
        .into()
}

/// The space a row's art takes: the server's icon or its scene, drawn at this size.
pub const ART_SIZE: Size = Size::new(145.0, 96.0);

/// Whether an address is one the page can save and join: it reads as a server address.
pub fn is_address(text: &str) -> bool {
    ServerAddress::parse(text).is_ok()
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/servers/mod.rs"]
mod tests;
