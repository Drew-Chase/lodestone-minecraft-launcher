//! Shared User-Agent construction for platform providers.
//!
//! Modrinth (and, by politeness, other APIs) asks clients to identify
//! themselves with `<name>/<version>`. We default to the crate's own
//! identity, but applications embedding this library should brand the UA
//! with their own name via each provider's `with_user_agent` constructor.

/// The default `User-Agent` string used by every provider when none is
/// supplied. Resolves to e.g. `"hopper-mc/0.1.0"`.
pub(crate) const DEFAULT_USER_AGENT: &str =
    concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// Build a `reqwest::Client` with the given User-Agent and gzip enabled.
/// Panics on construction failure — `reqwest::Client::builder()` can only
/// fail on misconfigured TLS features, which we don't toggle at runtime.
pub(crate) fn client_with_ua(user_agent: &str) -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(user_agent)
        .gzip(true)
        .build()
        .expect("reqwest client should build with default features")
}
