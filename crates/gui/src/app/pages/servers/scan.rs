//! The servers each instance's `servers.dat` lists, read for the Servers page on the blocking pool.

use crate::app::pages::instance::files::{self, ServerFile};
use crate::app::pages::worlds::Source;

/// The servers one instance's `servers.dat` lists.
#[derive(Debug, Clone)]
pub struct InstanceServers {
    /// The instance's name, which the page names the instances a server is listed in by.
    pub instance: String,
    /// The servers the list holds, in the list's order.
    pub servers: Vec<ServerFile>,
}

/// The servers of every source: one list per source that holds any, in the order the sources come.
pub fn read_all(sources: &[Source]) -> Vec<InstanceServers> {
    sources
        .iter()
        .map(|source| InstanceServers {
            instance: source.name.clone(),
            servers: files::scan_servers(&source.folder),
        })
        .filter(|list| !list.servers.is_empty())
        .collect()
}
