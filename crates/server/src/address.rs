//! A server's address as people type it: a host name or an IP address, with an optional port. A Minecraft server listens on
//! [`DEFAULT_PORT`] when its address names no port.

use std::fmt;

/// The port a Minecraft server listens on when its address names none.
pub const DEFAULT_PORT: u16 = 25565;

/// Why an address was not read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AddressError {
    /// The text was empty.
    #[error("the address is empty")]
    Empty,
    /// The port is not a number from 1 to 65535.
    #[error("the port is not a number from 1 to 65535")]
    Port,
    /// The host is not a host name or an IP address.
    #[error("the host is not a host name or an IP address")]
    Host,
}

/// A server's address: where to connect to it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServerAddress {
    /// A host name or an IP address, without brackets.
    pub host: String,
    /// The port the server listens on.
    pub port: u16,
}

impl ServerAddress {
    /// Reads `host`, `host:port`, or `[address]:port` for an IPv6 address. Without a port the address uses
    /// [`DEFAULT_PORT`].
    pub fn parse(text: &str) -> Result<Self, AddressError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(AddressError::Empty);
        }
        let (host, port) = if let Some(rest) = text.strip_prefix('[') {
            let close = rest.find(']').ok_or(AddressError::Host)?;
            let after = &rest[close + 1..];
            let port = if after.is_empty() {
                DEFAULT_PORT
            } else {
                parse_port(after.strip_prefix(':').ok_or(AddressError::Host)?)?
            };
            (&rest[..close], port)
        } else if text.matches(':').count() > 1 {
            // An IPv6 address written without brackets names no port.
            (text, DEFAULT_PORT)
        } else {
            match text.split_once(':') {
                Some((host, port)) => (host, parse_port(port)?),
                None => (text, DEFAULT_PORT),
            }
        };
        if !valid_host(host) {
            return Err(AddressError::Host);
        }
        Ok(Self { host: host.to_owned(), port })
    }
}

impl fmt::Display for ServerAddress {
    /// The address as it is written: IPv6 hosts in brackets, and the port only when it is not the default.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        if self.port == DEFAULT_PORT {
            write!(f, "{host}")
        } else {
            write!(f, "{host}:{}", self.port)
        }
    }
}

/// A port from 1 to 65535.
fn parse_port(text: &str) -> Result<u16, AddressError> {
    match text.trim().parse::<u16>() {
        Ok(port) if port > 0 => Ok(port),
        _ => Err(AddressError::Port),
    }
}

/// Whether `host` can be connected to: letters, digits, dots, hyphens and the colons of an IPv6 address, with no empty
/// label and no edge dot or hyphen.
fn valid_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | ':'))
        && !host.starts_with(['.', '-'])
        && !host.ends_with(['.', '-'])
        && !host.split('.').any(str::is_empty)
}

#[cfg(test)]
#[path = "../tests/address.rs"]
mod tests;
