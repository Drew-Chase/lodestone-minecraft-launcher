//! A modpack's install: the files the pack places and the archives it unpacks, the records of the mods and packs it names, and
//! the instance's look. Discover and the Import dialog both turn a parsed modpack into its install here, once they have the
//! pack's data and its archive on disk.

use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use lodestone_platforms::fingerprint::{self, Fingerprint};
use lodestone_platforms::{Modpack, ModpackFile};
use sha1::{Digest, Sha1};
use tokio::task::JoinSet;
use tracing::warn;

use super::modpack::{Payload, folder_prefix};
use super::package::random_scene;
use super::{IconSource, InstallRequest, new_id};
use crate::app::discover::{curseforge, resolve_curseforge_files};
use crate::app::mod_sources::{curseforge_sources, modrinth_records, pack_file, pack_record, stem};
use crate::app::pages::discover::{self, Source};
use crate::app::pages::library::instance::Loader;
use crate::app::pages::library::new_instance::naming;
use crate::app::paths::LauncherPaths;
use crate::app::shared_library::AddedContent;

/// How many CurseForge fingerprints are asked for at once.
const LOOKUPS_AT_ONCE: usize = 4;
/// How much of a pack is read at a time, so a large pack is never held in memory whole.
const READ_CHUNK: usize = 64 * 1024;

/// The platform page a modpack was taken from: its project there, and the project's page and picture when it has them.
/// The instance keeps the page and the project, so its own page can open the pack again.
pub(crate) struct Page {
    /// The platform the pack comes from.
    pub source: Source,
    /// The project's id on the platform.
    pub project_id: String,
    /// The project's page on the platform, when it has one.
    pub url: Option<String>,
    /// The project's picture on the platform. It becomes the instance's icon and banner, when it can be read.
    pub icon_url: Option<String>,
}

/// The release of a platform modpack an install takes: its id on the platform, and the version number the platform names it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// The release's id on the platform, which the instance keeps so it can tell the installed release apart.
    pub id: String,
    /// The release's version number, as the platform names it.
    pub number: String,
}

/// The install of `modpack`. `archive` is the pack's own file on disk, which its overrides come from. `source` is the
/// platform the pack is read as, which decides how its files are resolved and recorded. `page` is where the pack was
/// taken from, when it has a page. The folder is named after the pack and kept clear of `taken`.
pub(crate) async fn request(
    paths: &LauncherPaths,
    taken: &[String],
    modpack: Modpack,
    archive: Option<PathBuf>,
    source: Option<Source>,
    page: Option<Page>,
    release: Option<Release>,
) -> Result<InstallRequest, String> {
    let payload = payload_of(&modpack, archive, source).await?;
    let (biome, art_seed) = match &page {
        Some(page) => discover::scene_for(page.source, &page.project_id),
        None => random_scene(),
    };
    // The pack's own picture becomes the instance's icon and banner. Without one, the rose symbol and the scene stand in.
    let art = match page.as_ref().and_then(|page| page.icon_url.clone()) {
        Some(url) => discover::pack_art(url).await,
        None => None,
    };
    let (icon, banner) = match art {
        Some(art) => (IconSource::Uploaded(art.icon), Some(art.banner)),
        None => (
            IconSource::Composed {
                background: "rose".to_string(),
                symbol: "grass".to_string(),
            },
            None,
        ),
    };
    let directory_name = naming::unique_directory_name(&paths.instances(), &naming::clean_directory_name(&modpack.name), taken);
    Ok(InstallRequest {
        paths: paths.clone(),
        name: modpack.name,
        directory_name,
        mc_version: modpack.minecraft,
        loader: loader_of(modpack.loader),
        loader_version: modpack.loader_version,
        java_component: None,
        icon,
        biome,
        art_seed,
        banner,
        id: new_id(),
        source_url: page.as_ref().and_then(|page| page.url.clone()),
        source_platform: page.as_ref().map(|page| page.source.key().to_string()),
        source_project_id: page.map(|page| page.project_id),
        source_version: release.as_ref().map(|release| release.number.clone()),
        source_version_id: release.map(|release| release.id),
        modpack: Some(payload),
        settings: Default::default(),
        locked: false,
    })
}

/// The payload of a modpack: its files, its archives, and the platform records of the mods and packs it names. The files are
/// resolved to download links where the platform needs it. A pack read from a file (`source` is `None`) names no platform,
/// so its payload names nothing.
pub(crate) async fn payload_of(modpack: &Modpack, archive: Option<PathBuf>, source: Option<Source>) -> Result<Payload, String> {
    // A pack the overrides bundle is identified from its bytes: by its hash on Modrinth, and by its fingerprint on CurseForge
    // when a CurseForge key is built in. The archive is read for them only when the pack comes from a platform.
    let fingerprints = curseforge().is_ok();
    let bundled = match source {
        Some(Source::CurseForge | Source::Modrinth) => bundled_packs(archive.clone(), modpack.overrides.clone(), fingerprints).await,
        _ => Vec::new(),
    };
    let mut payload = Payload::from_modpack(modpack, archive);
    match source {
        Some(Source::CurseForge) => {
            payload.files = resolve_curseforge_files(payload.files).await?;
            payload.sources = curseforge_sources(&payload.files);
            // CurseForge names each bundled pack by its fingerprint. The packs it does not name are tried on Modrinth by hash.
            let (mut packs, unnamed) = curseforge_pack_records(bundled).await;
            packs.extend(modrinth_records(&hashed_packs(&unnamed)).await.packs);
            payload.packs = packs;
        }
        Some(Source::Modrinth) => {
            // The overrides lay over the files the pack names, so where both name a path, the bundled file is the one held.
            let mut hashed = hashed_files(&payload.files);
            hashed.retain(|(path, _)| !bundled.iter().any(|pack| &pack.path == path));
            hashed.extend(hashed_packs(&bundled));
            let records = modrinth_records(&hashed).await;
            // The packs Modrinth does not name are tried on CurseForge by their fingerprints.
            let named: HashSet<String> = records
                .packs
                .iter()
                .map(|record| format!("{}/{}", record.kind, record.file_name))
                .collect();
            let unnamed: Vec<BundledPack> = bundled.into_iter().filter(|pack| !named.contains(&pack.path)).collect();
            let (packs, _) = curseforge_pack_records(unnamed).await;
            payload.sources = records.mods;
            payload.packs = records.packs;
            payload.packs.extend(packs);
        }
        _ => {}
    }
    Ok(payload)
}

/// Each file a modpack places, with the path it goes to and the SHA-1 the pack gives it. A file with no hash cannot be
/// identified, so it is left out.
fn hashed_files(files: &[ModpackFile]) -> Vec<(String, String)> {
    files.iter().filter_map(|file| Some((file.path.clone(), file.sha1.clone()?))).collect()
}

/// Each bundled pack, with its path in the instance and the SHA-1 of its bytes, as a Modrinth lookup takes it.
fn hashed_packs(packs: &[BundledPack]) -> Vec<(String, String)> {
    packs.iter().map(|pack| (pack.path.clone(), pack.sha1.clone())).collect()
}

/// A resource or shader pack that a modpack's overrides bundle, read from the pack's archive.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BundledPack {
    /// The pack's path in the instance, such as `resourcepacks/Fresh.zip`.
    path: String,
    /// The SHA-1 of the pack's bytes, as lowercase hex.
    sha1: String,
    /// The CurseForge fingerprint of the pack's bytes, when one was asked for.
    fingerprint: Option<u32>,
}

/// The resource and shader packs a modpack's overrides bundle, read from the pack's archive. Only the packs are read, so the
/// other files of the archive cost nothing. A pack with no archive or no overrides folder has none. The fingerprints are read
/// only when `fingerprints` asks for them, since each costs a second read of its pack.
async fn bundled_packs(archive: Option<PathBuf>, overrides: Option<String>, fingerprints: bool) -> Vec<BundledPack> {
    let (Some(archive), Some(overrides)) = (archive, overrides) else {
        return Vec::new();
    };
    tokio::task::spawn_blocking(move || read_bundled_packs(&archive, &overrides, fingerprints))
        .await
        .unwrap_or_default()
}

/// The packs under `overrides` in `archive`, as `bundled_packs` names them. An entry that cannot be read is skipped, and an
/// archive that cannot be opened has none.
fn read_bundled_packs(archive: &Path, overrides: &str, fingerprints: bool) -> Vec<BundledPack> {
    let Ok(file) = std::fs::File::open(archive) else {
        return Vec::new();
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        warn!("reading the packs of the modpack {} failed", archive.display());
        return Vec::new();
    };
    let prefix = folder_prefix(overrides);
    let mut packs = Vec::new();
    for index in 0..zip.len() {
        let Some((path, sha1, kept)) = pack_digest(&mut zip, index, &prefix) else {
            continue;
        };
        let fingerprint = if fingerprints { pack_fingerprint(&mut zip, index, kept) } else { None };
        packs.push(BundledPack { path, sha1, fingerprint });
    }
    packs
}

/// The path in the instance, the SHA-1 and the number of kept bytes of the entry at `index` of `zip`, when it is a pack under
/// `prefix`.
fn pack_digest(zip: &mut zip::ZipArchive<std::fs::File>, index: usize, prefix: &str) -> Option<(String, String, u64)> {
    let mut entry = zip.by_index(index).ok()?;
    if entry.is_dir() {
        return None;
    }
    let name = entry.name().ok()?.into_owned();
    let relative = name.strip_prefix(prefix)?;
    pack_file(relative)?;
    let (sha1, kept) = digest_and_kept(&mut entry).ok()?;
    Some((relative.to_string(), sha1, kept))
}

/// The fingerprint of the entry at `index` of `zip`. It is read a second time, since a fingerprint starts from the number of
/// bytes it keeps, which the first read counted.
fn pack_fingerprint(zip: &mut zip::ZipArchive<std::fs::File>, index: usize, kept: u64) -> Option<u32> {
    let mut entry = zip.by_index(index).ok()?;
    let mut hasher = Fingerprint::new(kept);
    let mut buffer = vec![0u8; READ_CHUNK];
    loop {
        let read = entry.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Some(hasher.finish())
}

/// The SHA-1 of everything `reader` yields, as lowercase hex, and how many of its bytes a fingerprint keeps.
fn digest_and_kept(mut reader: impl Read) -> Result<(String, u64), String> {
    let mut hasher = Sha1::new();
    let mut kept = 0;
    let mut buffer = vec![0u8; READ_CHUNK];
    loop {
        let read = reader.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        kept += fingerprint::kept_len(&buffer[..read]);
    }
    Ok((format!("{:x}", hasher.finalize()), kept))
}

/// The records of the bundled packs CurseForge names by their fingerprints, and the packs it does not name, which are returned
/// to be tried elsewhere. A pack with no fingerprint, or whose lookup fails, is one CurseForge does not name. Without a
/// CurseForge key nothing is asked.
async fn curseforge_pack_records(packs: Vec<BundledPack>) -> (Vec<AddedContent>, Vec<BundledPack>) {
    let Ok(platform) = curseforge() else {
        return (Vec::new(), packs);
    };
    let mut records = Vec::new();
    let mut unnamed = Vec::new();
    let mut remaining = packs.into_iter();
    loop {
        let batch: Vec<BundledPack> = remaining.by_ref().take(LOOKUPS_AT_ONCE).collect();
        if batch.is_empty() {
            break;
        }
        let mut calls = JoinSet::new();
        for pack in batch {
            calls.spawn(async move {
                let named = match pack.fingerprint {
                    Some(fingerprint) => platform.fingerprint_match(fingerprint).await,
                    None => Ok(None),
                };
                (pack, named)
            });
        }
        while let Some(joined) = calls.join_next().await {
            let (pack, named) = match joined {
                Ok(answer) => answer,
                Err(error) => {
                    warn!("looking a modpack's pack up on CurseForge failed: {error}");
                    continue;
                }
            };
            match named {
                Ok(Some(found)) => {
                    let Some((folder, file_name)) = pack_file(&pack.path) else {
                        continue;
                    };
                    records.push(pack_record(
                        folder,
                        Source::CurseForge,
                        &found.project_id,
                        &found.file_id,
                        &stem(file_name),
                        file_name,
                    ));
                }
                Ok(None) => unnamed.push(pack),
                Err(error) => {
                    warn!("looking {} up on CurseForge failed: {error}", pack.path);
                    unnamed.push(pack);
                }
            }
        }
    }
    (records, unnamed)
}

/// The loader a modpack names, as an instance's loader. A pack with no loader is vanilla.
pub(crate) fn loader_of(name: Option<&str>) -> Loader {
    match name {
        Some("fabric") => Loader::Fabric,
        Some("forge") => Loader::Forge,
        Some("neoforge") => Loader::NeoForge,
        Some("quilt") => Loader::Quilt,
        _ => Loader::Vanilla,
    }
}

#[cfg(test)]
#[path = "../../../tests/app/install/pack.rs"]
mod tests;
