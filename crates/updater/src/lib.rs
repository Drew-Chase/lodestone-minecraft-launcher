//! The launcher's update check. It asks GitHub for the newest release, downloads the Windows installer, checks it
//! against the release's `SHA256SUMS.txt`, and runs it silently. The launcher decides when each step runs.

mod checksum;
mod download;
mod error;
mod install;
mod release;

pub use checksum::checksum_for;
pub use download::{download_verified, fetch_installer, verify_file};
pub use error::Error;
pub use install::{installed_with_installer, run_silent_elevated};
pub use release::{Asset, Release, installer_asset, latest, latest_for, newer};

/// The repository whose releases are checked.
pub const REPOSITORY: &str = "Drew-Chase/lodestone-minecraft-launcher";

/// The file each release publishes with the SHA-256 of its assets.
pub const CHECKSUMS: &str = "SHA256SUMS.txt";

/// The HTTP client for the update check. GitHub's API refuses requests that carry no User-Agent.
pub fn client(user_agent: &str) -> Result<reqwest::Client, Error> {
    Ok(lodestone_net::builder().user_agent(user_agent).build()?)
}
