//! The servers in `servers.dat`, the multiplayer list the game keeps in the instance folder. The game writes it as NBT: a
//! `servers` list of compounds, each with the server's `name`, its `ip` and, once the game has pinged it, its `icon` as
//! base64 PNG. This module only reads the list; the game owns the file.

use std::path::Path;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use iced::widget::image::Handle;
use serde::Deserialize;

use super::read_nbt;

/// The prefix an icon carries when it is written as a data URL, as a server's ping favicon is. The game writes its icons
/// without it.
const ICON_PREFIX: &str = "data:image/png;base64,";

/// One server the instance's list holds.
#[derive(Debug, Clone)]
pub struct ServerFile {
    /// The name the list gives the server, or its address when the list gives none.
    pub name: String,
    /// The address as the list holds it, such as `mc.hypixel.net` or `192.168.1.42:25565`.
    pub address: String,
    /// The icon the list keeps for the server, when it holds one that decodes.
    pub icon: Option<Handle>,
}

impl ServerFile {
    /// Whether the server's name or its address contains `query`, which is lower case. An empty query matches every server.
    pub fn matches(&self, query: &str) -> bool {
        query.is_empty() || self.name.to_lowercase().contains(query) || self.address.to_lowercase().contains(query)
    }
}

/// The servers `servers.dat` in `folder` lists, in the list's order. A file that is missing or does not decode lists none,
/// and an entry with no address is skipped.
pub fn scan(folder: &Path) -> Vec<ServerFile> {
    let Some(file) = read_nbt::<ServersFile>(&folder.join("servers.dat")) else {
        return Vec::new();
    };
    file.servers.into_iter().filter_map(from_entry).collect()
}

/// A server from its entry in the list, or `None` when the entry gives no address.
fn from_entry(entry: ServerEntry) -> Option<ServerFile> {
    let address = entry.ip.trim().to_owned();
    if address.is_empty() {
        return None;
    }
    let name = entry.name.trim().to_owned();
    Some(ServerFile {
        name: if name.is_empty() { address.clone() } else { name },
        icon: entry.icon.as_deref().and_then(decode_icon),
        address,
    })
}

/// The icon an entry holds, decoded from base64. `None` when it does not decode.
fn decode_icon(icon: &str) -> Option<Handle> {
    let encoded = icon.strip_prefix(ICON_PREFIX).unwrap_or(icon);
    STANDARD.decode(encoded.trim()).ok().map(Handle::from_bytes)
}

/// The top of `servers.dat`: the list sits under `servers`.
#[derive(Deserialize)]
struct ServersFile {
    #[serde(default)]
    servers: Vec<ServerEntry>,
}

/// One entry of the list. Each key is optional, since the game writes only what a server has.
#[derive(Deserialize)]
struct ServerEntry {
    #[serde(default)]
    name: String,
    #[serde(default)]
    ip: String,
    #[serde(default)]
    icon: Option<String>,
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/files/servers.rs"]
mod tests;
