//! The Servers page's work: reading the saved servers, pinging them, listening for games on the local network, and the
//! actions on a row. The page itself is `pages::servers`. The Add and Edit dialog is in `dialog`, and the Join Server dialog
//! in `join`.

pub mod dialog;
pub mod join;

use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Local;
use iced::futures::{Stream, StreamExt, stream};
use iced::widget::image;
use iced::{Subscription, Task};
use lodestone_server::address::ServerAddress;
use lodestone_server::lan::{self, Announcement};
use lodestone_server::status;
use tokio::sync::Semaphore;
use tracing::warn;

use super::components::alert::Alert;
use super::lang;
use super::pages::servers::{Message, Reply, RowAction, is_address};
use super::{LodestoneAppMessage, LodestoneLauncher};

/// How often the Servers page pings its servers while it is open.
pub const PING_EVERY: Duration = Duration::from_secs(10);
/// How long a server has to answer its status before it counts as offline.
const PING_TIMEOUT: Duration = Duration::from_secs(4);
/// How many servers are pinged at once, so a long list does not open a connection to each at the same moment.
const PINGS_AT_ONCE: usize = 8;

impl LodestoneLauncher {
    /// Opens the Servers page: its saved servers are read, and every one is pinged.
    pub(super) fn open_servers(&mut self) -> Task<LodestoneAppMessage> {
        Task::batch([self.load_saved_servers(), self.ping_servers()])
    }

    /// Reads the saved servers from the database.
    pub(super) fn load_saved_servers(&self) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        Task::perform(async move { db.saved_servers().await }, |result| {
            LodestoneAppMessage::Servers(Message::Loaded(result))
        })
    }

    /// Pings every saved server, a few at a time. Each server keeps its last answer until the new one comes back.
    fn ping_servers(&mut self) -> Task<LodestoneAppMessage> {
        let gate = Arc::new(Semaphore::new(PINGS_AT_ONCE));
        let tasks: Vec<_> = self
            .servers
            .addresses()
            .into_iter()
            .map(|address| {
                self.servers.begin_probe(&address);
                ping(address, gate.clone())
            })
            .collect();
        Task::batch(tasks)
    }

    /// Applies a message from the Servers page, or from a task the page started.
    pub(super) fn servers_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        match message {
            Message::Loaded(Ok(saved)) => {
                self.servers.set_saved(saved);
                self.ping_servers()
            }
            Message::Loaded(Err(error)) => {
                warn!("Could not read the saved servers: {error}");
                Task::none()
            }
            Message::Round => {
                self.servers.forget_quiet_games(Instant::now());
                self.ping_servers()
            }
            Message::Probed { address, result } => {
                self.servers.probed(address, result);
                Task::none()
            }
            Message::Heard(announcement) => {
                self.servers.heard(announcement, Instant::now());
                Task::none()
            }
            Message::Tab(_) | Message::Query(_) => {
                self.servers.update(message);
                Task::none()
            }
            Message::Submit => match self.servers.query().trim().to_owned() {
                text if is_address(&text) => self.open_servers_dialog(None, text),
                _ => Task::none(),
            },
            Message::Add => self.open_servers_dialog(None, String::new()),
            Message::RowMenu { address, index } => match RowAction::MENU.get(index) {
                Some(action) => self.row_action(address, *action),
                None => Task::none(),
            },
            Message::Join(address) => self.join_server(address),
            Message::ToggleFavorite(id) => match self.servers.server(id).cloned() {
                Some(server) => self.write_server(server.id, server.name, server.address, !server.favorite),
                None => Task::none(),
            },
            Message::SaveHeard { name, address } => self.save_new_server(name, address, false),
            Message::Written(Ok(())) => self.load_saved_servers(),
            Message::Written(Err(error)) => {
                warn!("Could not save the server: {error}");
                self.alerts.show(Alert::error(error)).discard()
            }
            Message::RemoveAnswered { id, confirmed } => match (confirmed, self.db.clone()) {
                (true, Some(db)) => Task::perform(async move { db.remove_server(id).await }, |result| {
                    LodestoneAppMessage::Servers(Message::Written(result))
                }),
                _ => Task::none(),
            },
        }
    }

    /// Carries out a row's menu action on the row at `address`.
    fn row_action(&mut self, address: String, action: RowAction) -> Task<LodestoneAppMessage> {
        let Some(server) = self.servers.saved().iter().find(|server| server.address == address).cloned() else {
            return Task::none();
        };
        match action {
            RowAction::Join => self.join_server(address),
            RowAction::Copy => iced::clipboard::write(address),
            RowAction::Favorite => self.write_server(server.id, server.name, server.address, !server.favorite),
            RowAction::Edit => self.open_servers_dialog(Some(server), String::new()),
            RowAction::Remove => {
                let message = lang::t_with("servers.remove.message", &[("name", server.name.as_str())]);
                let id = server.id;
                self.alerts
                    .show(Alert::danger(message).title(lang::t("servers.remove.title")))
                    .map(move |confirmed| LodestoneAppMessage::Servers(Message::RemoveAnswered { id, confirmed }))
            }
        }
    }

    /// Writes a saved server's name, address and favorite mark, then reads the list again.
    pub(super) fn write_server(&self, id: i64, name: String, address: String, favorite: bool) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        Task::perform(async move { db.update_server(id, &name, &address, favorite).await }, |result| {
            LodestoneAppMessage::Servers(Message::Written(result))
        })
    }

    /// Saves a new server under `address`, named `name`, then reads the list again.
    pub(super) fn save_new_server(&self, name: String, address: String, favorite: bool) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        let name = if name.trim().is_empty() { address.clone() } else { name };
        Task::perform(async move { db.add_server(&name, &address, favorite, Local::now()).await }, |result| {
            LodestoneAppMessage::Servers(Message::Written(result))
        })
    }
}

/// Pings the server at `address` once the gate lets it, and answers with what it said or why it did not answer.
fn ping(address: String, gate: Arc<Semaphore>) -> Task<LodestoneAppMessage> {
    Task::perform(
        async move {
            let _permit = gate.acquire_owned().await.ok();
            let result = match ServerAddress::parse(&address) {
                Ok(parsed) => status::ping(&parsed, PING_TIMEOUT)
                    .await
                    .map(|status| {
                        let icon = status.favicon.as_deref().and_then(status::favicon_png).map(image::Handle::from_bytes);
                        Reply { status, icon }
                    })
                    .map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            (address, result)
        },
        |(address, result)| LodestoneAppMessage::Servers(Message::Probed { address, result }),
    )
}

/// The games announced on the local network, one message each, for as long as the Servers page is open. A launcher that
/// cannot listen on the network yields none.
pub fn lan_games() -> impl Stream<Item = LodestoneAppMessage> {
    stream::once(lan::bind())
        .filter_map(|bound| {
            std::future::ready(match bound {
                Ok(socket) => Some(socket),
                Err(error) => {
                    warn!("Could not listen for games on the local network: {error}");
                    None
                }
            })
        })
        .flat_map(|socket| {
            stream::unfold(socket, |socket| async move {
                let announcement: Announcement = lan::next(&socket).await?;
                Some((LodestoneAppMessage::Servers(Message::Heard(announcement)), socket))
            })
        })
}

/// The Servers page's clocks: a round of pings every [`PING_EVERY`], and the games on the local network. Both run only while
/// the page is on screen.
pub fn subscription() -> Subscription<LodestoneAppMessage> {
    Subscription::batch([
        iced::time::every(PING_EVERY).map(|_| LodestoneAppMessage::Servers(Message::Round)),
        Subscription::run(lan_games),
    ])
}
