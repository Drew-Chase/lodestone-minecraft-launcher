//! The servers the Servers page lists: the ones the launcher saved, the ones an instance's `servers.dat` lists, and the games
//! announced on the local network now. A server that more than one holds is listed once, under the launcher's name and address,
//! and names the instances whose list it is in.

use std::collections::HashMap;

use iced::widget::image::Handle;
use lodestone_server::address::ServerAddress;
use lodestone_server::lan::Announcement;

use super::scan::InstanceServers;
use crate::app::database::servers::SavedServer;

/// A server on the Servers page.
#[derive(Debug, Clone)]
pub struct Listed {
    /// The address as it is written: the launcher's when it saved the server, else a list's, else the game's.
    pub address: String,
    /// The name: the launcher's when it saved the server, else a list's, else the game's title.
    pub name: String,
    /// The launcher's saved entry for the server, when the launcher saved it.
    pub saved: Option<SavedServer>,
    /// The icon the first list holds for the server, when one holds one.
    pub icon: Option<Handle>,
    /// The instances whose `servers.dat` lists the server, by name, in the order the instances come.
    pub instances: Vec<String>,
    /// Whether a game announces the server on the local network now.
    pub announced: bool,
}

impl Listed {
    /// Whether the launcher saved the server as a favorite.
    pub fn favorite(&self) -> bool {
        self.saved.as_ref().is_some_and(|saved| saved.favorite)
    }

    /// Whether the launcher knows the server: it saved it, or an instance's `servers.dat` lists it. A game that is only announced
    /// on the network is not known.
    pub fn known(&self) -> bool {
        self.saved.is_some() || !self.instances.is_empty()
    }
}

/// Merges the servers the launcher saved, the lists the instances hold and the games announced now, each address once. The
/// favorites come first, then the rest by name.
pub fn merge(saved: &[SavedServer], lists: &[InstanceServers], games: &[Announcement]) -> Vec<Listed> {
    let mut listed: Vec<Listed> = Vec::new();
    let mut at: HashMap<String, usize> = HashMap::new();
    for server in saved {
        at.insert(key(&server.address), listed.len());
        listed.push(Listed {
            address: server.address.clone(),
            name: server.name.clone(),
            saved: Some(server.clone()),
            icon: None,
            instances: Vec::new(),
            announced: false,
        });
    }
    for list in lists {
        for server in &list.servers {
            let index = *at.entry(key(&server.address)).or_insert_with(|| {
                listed.push(Listed {
                    address: server.address.clone(),
                    name: server.name.clone(),
                    saved: None,
                    icon: None,
                    instances: Vec::new(),
                    announced: false,
                });
                listed.len() - 1
            });
            let entry = &mut listed[index];
            if entry.icon.is_none() {
                entry.icon = server.icon.clone();
            }
            if !entry.instances.contains(&list.instance) {
                entry.instances.push(list.instance.clone());
            }
        }
    }
    for game in games {
        let address = game.address.to_string();
        let at_key = key(&address);
        if let Some(&index) = at.get(&at_key) {
            listed[index].announced = true;
        } else {
            at.insert(at_key, listed.len());
            listed.push(Listed {
                name: if game.title.trim().is_empty() {
                    address.clone()
                } else {
                    game.title.clone()
                },
                address,
                saved: None,
                icon: None,
                instances: Vec::new(),
                announced: true,
            });
        }
    }
    listed.sort_by_cached_key(|entry| (!entry.favorite(), entry.name.to_lowercase()));
    listed
}

/// The key two addresses share when they name the same server: the address as it reads, in lower case, with the default
/// port left out. An address that does not read is compared as it is written.
fn key(address: &str) -> String {
    match ServerAddress::parse(address) {
        Ok(parsed) => parsed.to_string().to_lowercase(),
        Err(_) => address.trim().to_lowercase(),
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/servers/listed.rs"]
mod tests;
