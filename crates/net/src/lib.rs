//! The network settings every request the launcher makes shares: the system proxy or a custom one, how long a connection
//! may stall, and how many idle connections a host keeps for reuse. The Settings page sets them, and each client is built
//! from them.
//!
//! The byte counter lives here too, so the speed the Network tab shows counts every download that reports progress.

use std::sync::PoisonError;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// The settings a client is built with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Whether the operating system's proxy is followed. When it is off, requests go direct, unless a custom proxy is set.
    pub use_system_proxy: bool,
    /// A proxy every request goes through, such as `http://proxy.example.com:3128`. `None` sends none.
    pub custom_proxy: Option<String>,
    /// How long a connection may take to open, and how long a read may stall, before the request fails.
    pub timeout: Duration,
    /// How many idle connections a host keeps open for reuse.
    pub idle_per_host: usize,
}

/// The settings before the Settings page has saved any: the system proxy, a 30 second timeout, and eight idle connections.
const DEFAULT: Config = Config {
    use_system_proxy: true,
    custom_proxy: None,
    timeout: Duration::from_secs(30),
    idle_per_host: 8,
};

/// The settings new clients are built with.
static CONFIG: RwLock<Config> = RwLock::new(DEFAULT);

/// Makes `config` the one new clients are built with. Clients already built keep the settings they were built with.
pub fn set(config: Config) {
    *CONFIG.write().unwrap_or_else(PoisonError::into_inner) = config;
}

/// The reqwest builder every client of the launcher starts from, with the current settings applied.
pub fn builder() -> reqwest::ClientBuilder {
    let config = CONFIG.read().unwrap_or_else(PoisonError::into_inner).clone();
    let mut builder = reqwest::Client::builder()
        .connect_timeout(config.timeout)
        .read_timeout(config.timeout)
        .pool_max_idle_per_host(config.idle_per_host);
    if !config.use_system_proxy {
        builder = builder.no_proxy();
    }
    if let Some(proxy) = config.custom_proxy.as_deref().and_then(|url| reqwest::Proxy::all(url).ok()) {
        builder = builder.proxy(proxy);
    }
    builder
}

/// A client built from [`builder`]. A client the settings cannot build falls back to reqwest's default one.
pub fn client() -> reqwest::Client {
    builder().build().unwrap_or_else(|_| reqwest::Client::new())
}

/// Every byte a download has reported receiving.
static BYTES: AtomicU64 = AtomicU64::new(0);

/// Records `count` more bytes received by a download.
pub fn record_bytes(count: u64) {
    BYTES.fetch_add(count, Ordering::Relaxed);
}

/// Every byte recorded so far. The Network tab takes the difference between two reads as the speed.
pub fn bytes_received() -> u64 {
    BYTES.load(Ordering::Relaxed)
}

#[cfg(test)]
#[path = "../tests/lib.rs"]
mod tests;
