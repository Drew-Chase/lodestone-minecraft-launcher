//! The shared files a version needs, each fetched only when it is missing: the vanilla version
//! JSON, the client jar, the libraries, the asset index and objects, the log config and the
//! natives. Installs and launches both call these, so a version that is already on disk costs
//! a check rather than a download.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use piston_mc::manifest_v2::ManifestV2;
use serde::Deserialize;
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard, mpsc};

use super::COMPLETE;
use super::download::{self, Missing, Part, PartReport};
use crate::app::lang;
use crate::app::launch::libraries::{Item, Native, Plan};
use crate::app::launch::manifest::{AssetIndex, Manifest};
use crate::app::paths::LauncherPaths;

/// The host Mojang serves asset objects from, by hash.
const ASSET_HOST: &str = "https://resources.download.minecraft.net";

/// The lock that serialises work on one shared folder. Installs and launches take it before they
/// write there, so the second one waits and then finds the files already on disk. Callers take
/// the locks in the order version, then Java, then loader, then natives, so no two of them wait on
/// each other.
pub async fn lock(key: String) -> OwnedMutexGuard<()> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Arc<AsyncMutex<()>>>>> = OnceLock::new();
    let locks = LOCKS.get_or_init(Default::default);
    let mutex = locks.lock().expect("the lock table is not poisoned").entry(key).or_default().clone();
    mutex.lock_owned().await
}

/// The vanilla version JSON for `mc`. It is read from the store when it is there, and otherwise
/// fetched from Mojang's list and saved exactly as it came.
pub async fn vanilla(paths: &LauncherPaths, client: &reqwest::Client, mc: &str) -> Result<Manifest, String> {
    let path = paths.version_json(mc);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(_) => {
            let text = fetch_version_json(client, mc).await?;
            write_file(&path, text.as_bytes())?;
            text
        }
    };
    Manifest::parse(&text).map_err(|error| lang::t_with("install.version_parse_failed", &[("mc", mc), ("error", error.as_str())]))
}

/// Downloads the version JSON Mojang lists for `mc`.
async fn fetch_version_json(client: &reqwest::Client, mc: &str) -> Result<String, String> {
    let manifest = ManifestV2::fetch()
        .await
        .map_err(|error| lang::t_with("install.version_list_failed", &[("error", error.to_string().as_str())]))?;
    let entry = manifest
        .versions
        .iter()
        .find(|entry| entry.id == mc)
        .ok_or_else(|| lang::t_with("install.version_missing", &[("mc", mc)]))?;
    get_text(client, &entry.url)
        .await
        .map_err(|error| lang::t_with("install.version_fetch_failed", &[("mc", mc), ("error", error.as_str())]))
}

/// Makes sure the client jar of `vanilla` is on disk, reporting as the client part.
pub async fn client_jar(
    paths: &LauncherPaths,
    client: &reqwest::Client,
    vanilla: &Manifest,
    reports: &mpsc::Sender<PartReport>,
) -> Result<(), String> {
    let file = vanilla
        .downloads
        .as_ref()
        .and_then(|downloads| downloads.client.as_ref())
        .ok_or_else(|| lang::t_with("install.client_missing", &[("mc", vanilla.id.as_str())]))?;
    let item = Item {
        url: Some(file.url.clone()),
        path: paths.client_jar(&vanilla.id),
        sha1: file.sha1.clone(),
        size: file.size,
    };
    let Missing { missing, present } = download::missing(vec![item]).await?;
    report(reports, Part::Client, present, 1).await;
    download::fetch(client, missing, Part::Client, present, 1, reports).await
}

/// Makes sure every library in `plan` is on disk, reporting as the libraries part.
pub async fn libraries(client: &reqwest::Client, plan: &Plan, reports: &mpsc::Sender<PartReport>) -> Result<(), String> {
    let total = plan.items.len();
    let Missing { missing, present } = download::missing(plan.items.clone()).await?;
    report(reports, Part::Libraries, present, total).await;
    download::fetch(client, missing, Part::Libraries, present, total, reports).await
}

/// Makes sure the asset index of `vanilla` and every object it names are on disk, reporting as
/// the assets part. The index is kept as Mojang wrote it, so its extra fields survive.
pub async fn assets(paths: &LauncherPaths, client: &reqwest::Client, vanilla: &Manifest, reports: &mpsc::Sender<PartReport>) -> Result<(), String> {
    let index = vanilla
        .asset_index
        .as_ref()
        .ok_or_else(|| lang::t_with("install.asset_index_missing", &[("mc", vanilla.id.as_str())]))?;
    let text = index_text(paths, client, index).await?;
    let items = object_items(paths, &index.id, &text)?;
    let total = items.len();
    let Missing { missing, present } = download::missing(items).await?;
    report(reports, Part::Assets, present, total).await;
    download::fetch(client, missing, Part::Assets, present, total, reports).await
}

/// The asset index text, from the store when it is there.
async fn index_text(paths: &LauncherPaths, client: &reqwest::Client, index: &AssetIndex) -> Result<String, String> {
    let path = paths.asset_indexes().join(format!("{}.json", index.id));
    if let Ok(text) = std::fs::read_to_string(&path) {
        return Ok(text);
    }
    let text = get_text(client, &index.url)
        .await
        .map_err(|error| lang::t_with("install.asset_index_failed", &[("id", index.id.as_str()), ("error", error.as_str())]))?;
    write_file(&path, text.as_bytes())?;
    Ok(text)
}

#[derive(Deserialize)]
struct IndexFile {
    objects: BTreeMap<String, IndexObject>,
}

#[derive(Deserialize)]
struct IndexObject {
    hash: String,
    size: u64,
}

/// One item per distinct object in the index, stored by hash under the shared objects folder. Two
/// names can share one hash, and two downloads never write the same file at once.
fn object_items(paths: &LauncherPaths, id: &str, text: &str) -> Result<Vec<Item>, String> {
    let index: IndexFile = serde_json::from_str(text)
        .map_err(|error| lang::t_with("install.asset_index_parse_failed", &[("id", id), ("error", error.to_string().as_str())]))?;
    let mut seen = std::collections::HashSet::new();
    Ok(index
        .objects
        .into_values()
        .filter_map(|object| {
            if !seen.insert(object.hash.clone()) {
                return None;
            }
            let prefix = object.hash.get(..2)?.to_string();
            Some(Item {
                url: Some(format!("{ASSET_HOST}/{prefix}/{}", object.hash)),
                path: paths.asset_objects().join(&prefix).join(&object.hash),
                sha1: Some(object.hash),
                size: Some(object.size),
            })
        })
        .collect())
}

/// Makes sure the log4j config `vanilla` names is on disk and returns its path.
pub async fn log_config(paths: &LauncherPaths, client: &reqwest::Client, vanilla: &Manifest) -> Result<Option<PathBuf>, String> {
    let Some(logging) = vanilla.logging.as_ref() else {
        return Ok(None);
    };
    let file = &logging.client.file;
    let path = paths.log_configs().join(&file.id);
    let item = Item {
        url: Some(file.url.clone()),
        path: path.clone(),
        sha1: file.sha1.clone(),
        size: file.size,
    };
    let Missing { missing, .. } = download::missing(vec![item]).await?;
    download::fetch_quiet(client, missing).await?;
    Ok(Some(path))
}

/// Extracts the natives in `natives` into `dir`, unless a previous launch already did. The marker
/// written afterwards is what the next launch checks. The folder's lock is held throughout, so a
/// launch that starts while another is extracting waits for it, then finds the marker and skips.
pub async fn natives(dir: PathBuf, natives: Vec<Native>) -> Result<(), String> {
    let _extracting = lock(natives_key(&dir)).await;
    tokio::task::spawn_blocking(move || extract_natives(&dir, &natives))
        .await
        .map_err(|error| error.to_string())?
}

/// The lock key of one natives folder.
fn natives_key(dir: &Path) -> String {
    format!("natives:{}", dir.display())
}

fn extract_natives(dir: &Path, natives: &[Native]) -> Result<(), String> {
    if dir.join(COMPLETE).is_file() {
        return Ok(());
    }
    std::fs::create_dir_all(dir).map_err(|error| {
        lang::t_with(
            "install.could_not_create",
            &[("path", dir.display().to_string().as_str()), ("error", error.to_string().as_str())],
        )
    })?;
    for native in natives {
        extract_archive(&native.archive, dir, &native.exclude)?;
    }
    std::fs::write(dir.join(COMPLETE), b"").map_err(|error| {
        lang::t_with(
            "install.could_not_finish",
            &[("path", dir.display().to_string().as_str()), ("error", error.to_string().as_str())],
        )
    })
}

/// Copies every file of one archive into `dir`, leaving out directories and any path that starts
/// with an excluded prefix.
fn extract_archive(archive: &Path, dir: &Path, exclude: &[String]) -> Result<(), String> {
    let failed = |error: String| {
        lang::t_with(
            "install.natives_failed",
            &[("archive", archive.display().to_string().as_str()), ("error", error.as_str())],
        )
    };
    let file = std::fs::File::open(archive).map_err(|error| failed(error.to_string()))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|error| failed(error.to_string()))?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).map_err(|error| failed(error.to_string()))?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let listed = name.to_string_lossy().replace('\\', "/");
        if entry.is_dir() || exclude.iter().any(|prefix| listed.starts_with(prefix.as_str())) {
            continue;
        }
        let target = dir.join(&name);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| failed(error.to_string()))?;
        }
        let mut out = std::fs::File::create(&target).map_err(|error| failed(error.to_string()))?;
        std::io::copy(&mut entry, &mut out).map_err(|error| failed(error.to_string()))?;
    }
    Ok(())
}

/// Reports a part's count.
async fn report(reports: &mpsc::Sender<PartReport>, part: Part, done: usize, total: usize) {
    let _ = reports.send(PartReport { part, done, total }).await;
}

/// The body of a GET, failing on an error status.
async fn get_text(client: &reqwest::Client, url: &str) -> Result<String, String> {
    client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| error.to_string())?
        .text()
        .await
        .map_err(|error| error.to_string())
}

/// Writes `bytes` to `path` through a `.part` file, so an interrupted write is never mistaken
/// for a finished one.
pub fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            lang::t_with(
                "install.could_not_create",
                &[("path", parent.display().to_string().as_str()), ("error", error.to_string().as_str())],
            )
        })?;
    }
    let mut partial = path.as_os_str().to_owned();
    partial.push(".part");
    let partial = PathBuf::from(partial);
    std::fs::write(&partial, bytes).map_err(|error| {
        lang::t_with(
            "install.could_not_write",
            &[("path", partial.display().to_string().as_str()), ("error", error.to_string().as_str())],
        )
    })?;
    std::fs::rename(&partial, path).map_err(|error| {
        lang::t_with(
            "install.could_not_replace",
            &[("path", path.display().to_string().as_str()), ("error", error.to_string().as_str())],
        )
    })
}

#[cfg(test)]
#[path = "../../../tests/app/install/ensure.rs"]
mod tests;
