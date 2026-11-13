//! The network settings every request the launcher makes shares: the system proxy or a custom one, how long a connection
//! may stall, and how many idle connections a host keeps for reuse. The Settings page sets them, and each client is built
//! from them.
//!
//! The byte counter lives here too, so the speed the Network tab shows counts every download that reports progress.

pub mod download;

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

/// The settings new clients are built with now. A client kept for reuse compares them with its own, to know when to rebuild.
pub fn current() -> Config {
    CONFIG.read().unwrap_or_else(PoisonError::into_inner).clone()
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
    if let Some(url) = config.custom_proxy.as_deref() {
        match reqwest::Proxy::all(url) {
            Ok(proxy) => builder = builder.proxy(proxy),
            // The Settings page marks the address as unusable (see [`proxy_is_valid`]), so this is only logged.
            Err(error) => tracing::warn!("the custom proxy {url:?} cannot be used, so requests skip it: {error}"),
        }
    }
    builder
}

/// A client built from [`builder`]. A client the settings cannot build falls back to reqwest's default one.
pub fn client() -> reqwest::Client {
    builder().build().unwrap_or_else(|error| {
        // The fallback keeps the timeout, so a stalled request still ends. It drops the proxy settings, which is why it is logged.
        tracing::warn!("the network settings could not build a client, so a client with the default timeout is used: {error}");
        let timeout = CONFIG.read().unwrap_or_else(PoisonError::into_inner).timeout;
        reqwest::Client::builder()
            .connect_timeout(timeout)
            .read_timeout(timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

/// Whether `url` is a proxy address a client can use, such as `http://proxy.example.com:3128`.
pub fn proxy_is_valid(url: &str) -> bool {
    reqwest::Proxy::all(url.trim()).is_ok()
}

/// GETs `url` with `client` and reads the body as JSON. An answer that is not a success is an error.
pub async fn get_json<T: serde::de::DeserializeOwned>(client: &reqwest::Client, url: &str) -> Result<T, reqwest::Error> {
    client.get(url).send().await?.error_for_status()?.json::<T>().await
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
