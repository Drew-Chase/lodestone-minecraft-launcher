//! Downloads a release's file and checks it against the release's checksum before anything unpacks or runs it.

use std::path::{Path, PathBuf};

use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::CHECKSUMS;
use crate::checksum::checksum_for;
use crate::error::{Error, io_error};
use crate::release::{Asset, Release};

/// Downloads the release's file called `name` into `dir`, after checking it against the release's checksum list. Returns
/// the path of the verified file.
pub async fn fetch_asset(client: &Client, release: &Release, name: &str, dir: &Path) -> Result<PathBuf, Error> {
    let asset = release
        .assets
        .iter()
        .find(|candidate| candidate.name == name)
        .ok_or_else(|| Error::MissingAsset(name.to_string()))?;
    let sums_asset = release
        .assets
        .iter()
        .find(|candidate| candidate.name == CHECKSUMS)
        .ok_or_else(|| Error::MissingAsset(CHECKSUMS.to_string()))?;
    let sums = client
        .get(&sums_asset.browser_download_url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let expected = checksum_for(&sums, &asset.name).ok_or_else(|| Error::MissingAsset(format!("a checksum for {}", asset.name)))?;
    download_verified(client, asset, &expected, dir).await
}

/// Streams `asset` into `dir` and checks its SHA-256 against `expected`, as lowercase hex. The bytes go to a `.part`
/// file that is renamed only once the hash matches, so an interrupted download never looks finished.
async fn download_verified(client: &Client, asset: &Asset, expected: &str, dir: &Path) -> Result<PathBuf, Error> {
    tokio::fs::create_dir_all(dir).await.map_err(|source| io_error(dir, source))?;
    let target = dir.join(&asset.name);
    let partial = dir.join(format!("{}.part", asset.name));

    let mut response = client.get(&asset.browser_download_url).send().await?.error_for_status()?;
    let mut file = tokio::fs::File::create(&partial).await.map_err(|source| io_error(&partial, source))?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await.map_err(|source| io_error(&partial, source))?;
    }
    file.flush().await.map_err(|source| io_error(&partial, source))?;
    drop(file);

    // Hashing a file this size would hold a runtime thread for a while, so it runs on the blocking pool.
    let (to_check, expected) = (partial.clone(), expected.to_string());
    let matches = tokio::task::spawn_blocking(move || verify_file(&to_check, &expected))
        .await
        .map_err(|error| io_error(&partial, std::io::Error::other(error)))?
        .map_err(|source| io_error(&partial, source))?;
    if !matches {
        let _ = tokio::fs::remove_file(&partial).await;
        return Err(Error::Checksum);
    }
    tokio::fs::rename(&partial, &target).await.map_err(|source| io_error(&partial, source))?;
    Ok(target)
}

/// Whether the file at `path` hashes to `expected`, in either case of hex.
pub fn verify_file(path: &Path, expected: &str) -> std::io::Result<bool> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()).eq_ignore_ascii_case(expected))
}

#[cfg(test)]
#[path = "../tests/download.rs"]
mod tests;
