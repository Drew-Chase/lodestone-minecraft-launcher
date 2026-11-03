//! The Server List Ping: the status a Minecraft server answers with, read over its TCP port. The client sends a handshake
//! that asks for the status, requests the status, and then pings to time the round trip. The status is JSON: the version,
//! the players, the message of the day, and an optional icon.

use std::time::{Duration, Instant};

use base64::Engine as _;
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::address::ServerAddress;
use crate::chat;
use crate::protocol;

/// The protocol number the handshake announces. A server answers the status request whatever number it names, so this is
/// a recent one.
const PROTOCOL: i32 = 767;
/// The packet ids the exchange uses. The handshake and the status request both have id 0, in different states.
const HANDSHAKE: i32 = 0x00;
const STATUS_REQUEST: i32 = 0x00;
const PING: i32 = 0x01;
/// The state the handshake asks the server to move into: the status.
const NEXT_STATE_STATUS: i32 = 1;
/// The most a packet from the server may be. A status carries an icon of a few kilobytes, so this is generous.
const MAX_PACKET: usize = 1 << 20;
/// How long the ping may take on its own. A server that answers its status but not a ping still reports its status.
const PING_TIMEOUT: Duration = Duration::from_secs(1);
/// The payload the ping carries. The server sends it back unchanged, so any value serves.
const PING_PAYLOAD: i64 = 0x4C_6F_64_65_73_74;
/// The prefix of an icon the status carries, before its base64 bytes.
const ICON_PREFIX: &str = "data:image/png;base64,";

/// What a server said about itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// The version the server names, such as "1.21.4".
    pub version: String,
    /// The protocol number the server speaks.
    pub protocol: i32,
    /// How many players are on, and the most the server allows.
    pub online: u32,
    pub max: u32,
    /// The message of the day, as plain text.
    pub motd: String,
    /// The server's icon, as a `data:image/png;base64,` address, when it has one.
    pub favicon: Option<String>,
    /// How long the server took to answer the ping.
    pub latency: Duration,
}

/// Why a server did not give a status.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PingError {
    /// The connection could not be made, or was refused, or dropped.
    #[error("could not reach the server: {0}")]
    Unreachable(String),
    /// The server did not answer in time.
    #[error("the server did not answer in time")]
    Timeout,
    /// The server answered, but not with a status.
    #[error("the server did not send a status: {0}")]
    Malformed(String),
}

/// Asks the server at `address` for its status. The whole exchange must finish within `timeout`.
pub async fn ping(address: &ServerAddress, timeout: Duration) -> Result<Status, PingError> {
    tokio::time::timeout(timeout, exchange(address)).await.map_err(|_| PingError::Timeout)?
}

/// The exchange itself: the handshake, the status request and its answer, then the ping.
async fn exchange(address: &ServerAddress) -> Result<Status, PingError> {
    let mut stream = TcpStream::connect((address.host.as_str(), address.port)).await.map_err(connect_error)?;
    let mut handshake = Vec::new();
    protocol::write_varint(&mut handshake, PROTOCOL);
    protocol::write_string(&mut handshake, &address.host);
    handshake.extend_from_slice(&address.port.to_be_bytes());
    protocol::write_varint(&mut handshake, NEXT_STATE_STATUS);
    stream.write_all(&protocol::frame(HANDSHAKE, &handshake)).await.map_err(connect_error)?;

    let requested = Instant::now();
    stream.write_all(&protocol::frame(STATUS_REQUEST, &[])).await.map_err(connect_error)?;
    let (id, body) = read_packet(&mut stream).await?;
    if id != STATUS_REQUEST {
        return Err(PingError::Malformed(format!("packet {id} answers the status request")));
    }
    let json = read_string(&body)?;
    let mut status = from_json(&json)?;
    let status_round_trip = requested.elapsed();

    let pinged = Instant::now();
    stream
        .write_all(&protocol::frame(PING, &PING_PAYLOAD.to_be_bytes()))
        .await
        .map_err(connect_error)?;
    status.latency = match tokio::time::timeout(PING_TIMEOUT, read_packet(&mut stream)).await {
        Ok(Ok((PING, _))) => pinged.elapsed(),
        _ => status_round_trip,
    };
    Ok(status)
}

/// The status a server's JSON describes. Its latency is not in the JSON, so it is zero until the ping sets it.
pub fn from_json(text: &str) -> Result<Status, PingError> {
    let json: Value = serde_json::from_str(text).map_err(|error| PingError::Malformed(error.to_string()))?;
    let version = &json["version"];
    let players = &json["players"];
    Ok(Status {
        version: version["name"].as_str().unwrap_or_default().to_owned(),
        protocol: version["protocol"]
            .as_i64()
            .and_then(|protocol| i32::try_from(protocol).ok())
            .unwrap_or_default(),
        online: players["online"].as_u64().and_then(|count| u32::try_from(count).ok()).unwrap_or_default(),
        max: players["max"].as_u64().and_then(|count| u32::try_from(count).ok()).unwrap_or_default(),
        motd: chat::plain(&json["description"]),
        favicon: json["favicon"].as_str().map(str::to_owned),
        latency: Duration::ZERO,
    })
}

/// The icon's PNG bytes, from the `data:image/png;base64,` address a status carries. `None` for any other address, or for
/// bytes that are not base64.
pub fn favicon_png(favicon: &str) -> Option<Vec<u8>> {
    let encoded = favicon.strip_prefix(ICON_PREFIX)?;
    base64::engine::general_purpose::STANDARD.decode(encoded.trim()).ok()
}

/// Reads one packet: its id and its body. A packet longer than [`MAX_PACKET`], or a length that does not read, is an error.
async fn read_packet(stream: &mut TcpStream) -> Result<(i32, Vec<u8>), PingError> {
    let mut length_bytes = Vec::with_capacity(5);
    loop {
        let byte = stream.read_u8().await.map_err(io_error)?;
        length_bytes.push(byte);
        if byte & 0x80 == 0 {
            break;
        }
        if length_bytes.len() == 5 {
            return Err(PingError::Malformed("the packet length runs past five bytes".to_owned()));
        }
    }
    let (length, _) = protocol::read_varint(&length_bytes).ok_or_else(|| PingError::Malformed("the packet length does not read".to_owned()))?;
    let length = usize::try_from(length)
        .ok()
        .filter(|length| (1..=MAX_PACKET).contains(length))
        .ok_or_else(|| PingError::Malformed("the packet is empty or too long".to_owned()))?;
    let mut payload = vec![0; length];
    stream.read_exact(&mut payload).await.map_err(io_error)?;
    let (id, used) = protocol::read_varint(&payload).ok_or_else(|| PingError::Malformed("the packet has no id".to_owned()))?;
    let body = payload.split_off(used);
    Ok((id, body))
}

/// Reads a string from the start of a packet body: its length in bytes as a VarInt, then its UTF-8 bytes.
fn read_string(body: &[u8]) -> Result<String, PingError> {
    let (length, used) = protocol::read_varint(body).ok_or_else(|| PingError::Malformed("the status has no length".to_owned()))?;
    let length = usize::try_from(length).map_err(|_| PingError::Malformed("the status has a negative length".to_owned()))?;
    let end = used
        .checked_add(length)
        .ok_or_else(|| PingError::Malformed("the status length overflows".to_owned()))?;
    let bytes = body
        .get(used..end)
        .ok_or_else(|| PingError::Malformed("the status is cut short".to_owned()))?;
    String::from_utf8(bytes.to_vec()).map_err(|_| PingError::Malformed("the status is not UTF-8".to_owned()))
}

fn connect_error(error: std::io::Error) -> PingError {
    PingError::Unreachable(error.to_string())
}

fn io_error(error: std::io::Error) -> PingError {
    if error.kind() == std::io::ErrorKind::UnexpectedEof {
        PingError::Malformed("the server closed the connection".to_owned())
    } else {
        PingError::Unreachable(error.to_string())
    }
}

#[cfg(test)]
#[path = "../tests/status.rs"]
mod tests;
