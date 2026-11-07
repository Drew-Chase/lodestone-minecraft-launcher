//! The launcher's update check. It asks GitHub for the newest release, downloads the file that this system installs from,
//! and checks it against the release's `SHA256SUMS.txt` before anything unpacks or runs it. The launcher decides when each
//! step runs: the download in the background, and the install once the user asks for it.

mod apply;
mod checksum;
mod download;
mod error;
mod install;
mod release;

pub use apply::{can_write_to, extract_bundle, extract_executable, relaunch_after_exit, replace_bundle, replace_executable};
pub use checksum::checksum_for;
pub use download::{fetch_asset, verify_file};
pub use error::Error;
pub use install::{installed_with_installer, run_silent_elevated};
pub use release::{Answer, Asset, Payload, Release, check, newer};

/// The repository whose releases are checked.
pub const REPOSITORY: &str = "Drew-Chase/lodestone-minecraft-launcher";

/// The file each release publishes with the SHA-256 of its assets.
pub const CHECKSUMS: &str = "SHA256SUMS.txt";

/// The HTTP client for the update check. GitHub's API refuses requests that carry no User-Agent.
pub fn client(user_agent: &str) -> Result<reqwest::Client, Error> {
    Ok(lodestone_net::builder().user_agent(user_agent).build()?)
}
