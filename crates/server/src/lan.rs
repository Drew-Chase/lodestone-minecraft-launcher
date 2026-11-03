//! The games announced on the local network. A Minecraft game that is open to the local network sends a multicast message
//! every few seconds to [`GROUP`] on [`PORT`]. The message names the game's title and the port it listens on, as
//! `[MOTD]title[/MOTD][AD]port[/AD]`, and the address to join it at is the sender's address with that port.

use std::net::{Ipv4Addr, SocketAddrV4};

use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;

use crate::address::ServerAddress;
use crate::chat;

/// The multicast group the games announce on.
pub const GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 2, 60);
/// The port the announcements are sent to.
pub const PORT: u16 = 4445;

/// A game on the local network: its title, and the address to join it at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announcement {
    pub title: String,
    pub address: ServerAddress,
}

/// Reads an announcement from the text of a packet that `sender` sent. A packet that is not an announcement is `None`.
pub fn parse(text: &str, sender: &str) -> Option<Announcement> {
    let title = between(text, "[MOTD]", "[/MOTD]")?;
    let port = between(text, "[AD]", "[/AD]")?.trim().parse::<u16>().ok().filter(|port| *port > 0)?;
    Some(Announcement {
        title: chat::strip_codes(title),
        address: ServerAddress {
            host: sender.to_owned(),
            port,
        },
    })
}

/// The text between the first `start` and the `end` after it.
fn between<'a>(text: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let from = text.find(start)? + start.len();
    let to = text[from..].find(end)? + from;
    Some(&text[from..to])
}

/// Joins the announcement group on the local network. Every game on the machine listens on the same port, so the socket
/// shares it.
pub async fn bind() -> std::io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(all(unix, not(any(target_os = "solaris", target_os = "illumos", target_os = "cygwin"))))]
    socket.set_reuse_port(true)?;
    socket.set_nonblocking(true)?;
    socket.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, PORT).into())?;
    let socket = std::net::UdpSocket::from(socket);
    socket.join_multicast_v4(&GROUP, &Ipv4Addr::UNSPECIFIED)?;
    UdpSocket::from_std(socket)
}

/// Waits for the next announcement, skipping packets that are not one. `None` once the socket fails.
pub async fn next(socket: &UdpSocket) -> Option<Announcement> {
    let mut buffer = [0u8; 1024];
    loop {
        let (length, sender) = socket.recv_from(&mut buffer).await.ok()?;
        if let Ok(text) = std::str::from_utf8(&buffer[..length])
            && let Some(announcement) = parse(text, &sender.ip().to_string())
        {
            return Some(announcement);
        }
    }
}

#[cfg(test)]
#[path = "../tests/lan.rs"]
mod tests;
