//! Minecraft's network protocols for the launcher's Servers page: the status a server answers with (the Server List Ping),
//! the games announced on the local network, and the addresses people type to name a server. Nothing here touches the
//! interface, so the protocols can be tested on their own.

pub mod address;
pub mod chat;
pub mod lan;
pub mod protocol;
pub mod status;
