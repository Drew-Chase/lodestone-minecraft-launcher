//! Where the mods an instance holds came from, and what is newer for them. A modpack names each jar it places in `mods/`,
//! and each name becomes a record of the platform project and release the jar was installed as. The Mods tab checks the
//! jars it has no record of by their hash, and checks each recorded mod for a newer release. These are the platform calls
//! behind all of that, and the answers are written to the database and the hash cache here.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use lodestone_platforms::{ContentType, ContentVersion, ModpackFile};
use sha1::{Digest, Sha1};
use tokio::task::JoinSet;
use tracing::warn;

use crate::app::database::Database;
use crate::app::discover::{MODRINTH, curseforge};
use crate::app::install::mods::fitting;
use crate::app::pages::discover::Source;
use crate::app::perf;
use crate::app::shared_library::{self, AddedContent, JarHash, ModSource};

/// How many CurseForge projects are asked for their releases at once.
const CURSEFORGE_AT_ONCE: usize = 4;

/// The name of a file that sits directly in `folder` of a modpack, such as a jar in `mods/`, or `None` for any other path.
pub(crate) fn direct_child<'a>(path: &'a str, folder: &str) -> Option<&'a str> {
    let name = path.strip_prefix(folder)?.strip_prefix('/')?;
    (!name.is_empty() && !name.contains('/')).then_some(name)
}

/// The name of a jar that sits directly in a modpack's `mods/` folder, or `None` for any other path.
pub(crate) fn mod_jar_name(path: &str) -> Option<&str> {
    direct_child(path, shared_library::folder_name(ContentType::Mod))
}

/// A jar's name without its extension: the name a record shows when the platform's is not known.
pub(crate) fn stem(file_name: &str) -> String {
    file_name.rsplit_once('.').map_or(file_name, |(stem, _)| stem).to_string()
}

/// The records a Modrinth modpack's files give. Its mods are recorded against the instance's `mods/` folder, and its packs
/// as the instance's content, under the folder kind each pack is in.
#[derive(Debug, Default)]
pub(crate) struct ModrinthRecords {
    /// The mods, each with the project and release its jar was installed as.
    pub mods: Vec<ModSource>,
    /// The packs, each with the project and release it was installed as.
    pub packs: Vec<AddedContent>,
}

/// A file a modpack places in one of the recorded folders, with the folder and the hash the pack gives it, in lowercase.
struct Placed<'a> {
    folder: &'static str,
    file_name: &'a str,
    sha1: String,
}

/// The project and release a hash names on Modrinth.
struct Release {
    project_id: String,
    version_id: String,
}

/// The folders whose files a modpack records: the mods' folder, and the folder of each kind of pack.
fn recorded_folders() -> [&'static str; 3] {
    [
        shared_library::folder_name(ContentType::Mod),
        shared_library::folder_name(ContentType::ResourcePack),
        shared_library::folder_name(ContentType::Shader),
    ]
}

/// The name of a pack a modpack or package keeps directly in a resource or shader folder, with that kind's folder, or `None`
/// for any other path.
pub(crate) fn pack_file(path: &str) -> Option<(&'static str, &str)> {
    [ContentType::ResourcePack, ContentType::Shader].into_iter().find_map(|kind| {
        let folder = shared_library::folder_name(kind);
        Some((folder, direct_child(path, folder)?))
    })
}

/// The records of a Modrinth modpack's mods and packs, from each file's path in the instance and the SHA-1 of its bytes. A
/// file's SHA-1 names its release on Modrinth, and the release names its project. A file Modrinth does not know gets no
/// record, and nor does one whose release names no project. A failed lookup is logged and leaves the files unrecorded, so
/// the install still runs.
pub(crate) async fn modrinth_records(hashed: &[(String, String)]) -> ModrinthRecords {
    let folders = recorded_folders();
    let placed: Vec<Placed> = hashed
        .iter()
        .filter_map(|(path, sha1)| {
            let (folder, file_name) = folders.iter().find_map(|folder| Some((*folder, direct_child(path, folder)?)))?;
            Some(Placed {
                folder,
                file_name,
                sha1: sha1.to_ascii_lowercase(),
            })
        })
        .collect();
    if placed.is_empty() {
        return ModrinthRecords::default();
    }
    let hashes: Vec<String> = placed.iter().map(|file| file.sha1.clone()).collect();
    let found = match MODRINTH.versions_by_hashes(&hashes).await {
        Ok(found) => found,
        Err(error) => {
            warn!("identifying the modpack's files on Modrinth failed: {error}");
            return ModrinthRecords::default();
        }
    };
    let releases: HashMap<String, Release> = found
        .into_iter()
        .filter_map(|(sha1, release)| {
            let project_id = release.project_id?;
            Some((
                sha1,
                Release {
                    project_id,
                    version_id: release.id,
                },
            ))
        })
        .collect();
    let project_ids: Vec<String> = releases
        .values()
        .map(|release| release.project_id.clone())
        .collect::<HashSet<String>>()
        .into_iter()
        .collect();
    let names = match MODRINTH.project_names(&project_ids).await {
        Ok(names) => names,
        Err(error) => {
            warn!("reading the names of the modpack's files on Modrinth failed: {error}");
            Default::default()
        }
    };
    records_of(&placed, &releases, &names)
}

/// The records the releases name for the placed files: each mod a record of its jar, and each pack a record under the folder
/// kind it is in. The name is the project's when the platform gives one, and the file's own otherwise.
fn records_of(placed: &[Placed], releases: &HashMap<String, Release>, names: &HashMap<String, String>) -> ModrinthRecords {
    let mut records = ModrinthRecords::default();
    for file in placed {
        let Some(release) = releases.get(&file.sha1) else {
            continue;
        };
        let name = names.get(&release.project_id).cloned().unwrap_or_else(|| stem(file.file_name));
        if file.folder == shared_library::folder_name(ContentType::Mod) {
            records.mods.push(ModSource {
                instance: String::new(),
                platform: Source::Modrinth.key().to_string(),
                project_id: release.project_id.clone(),
                version_id: release.version_id.clone(),
                name,
                file_name: file.file_name.to_string(),
                sha1: Some(file.sha1.clone()),
            });
        } else {
            records.packs.push(pack_record(
                file.folder,
                Source::Modrinth,
                &release.project_id,
                &release.version_id,
                &name,
                file.file_name,
            ));
        }
    }
    records
}

/// The record of a pack a modpack places in `folder`, a kind's folder, as the release of `platform` it was installed as. The
/// record names no instance until the install has one.
pub(crate) fn pack_record(folder: &str, platform: Source, project_id: &str, version_id: &str, name: &str, file_name: &str) -> AddedContent {
    AddedContent {
        instance: String::new(),
        scope: String::new(),
        kind: folder.to_string(),
        platform: platform.key().to_string(),
        project_id: project_id.to_string(),
        version_id: version_id.to_string(),
        name: name.to_string(),
        file_name: file_name.to_string(),
    }
}

/// The records of a CurseForge modpack's mods. Each file names its project and file, and its jar was named when the
/// install resolved it. The jar's name stands in for the project's, which the manifest does not give.
pub(crate) fn curseforge_sources(files: &[ModpackFile]) -> Vec<ModSource> {
    files
        .iter()
        .filter_map(|file| {
            let file_name = mod_jar_name(&file.path)?;
            Some(ModSource {
                instance: String::new(),
                platform: Source::CurseForge.key().to_string(),
                project_id: file.project_id.clone()?,
                version_id: file.file_id.clone()?,
                name: stem(file_name),
                file_name: file_name.to_string(),
                sha1: file.sha1.clone(),
            })
        })
        .collect()
}

/// What a check needs about an instance: its id, its folder, and the loader and Minecraft version its releases run on.
#[derive(Debug, Clone)]
pub struct CheckContext {
    pub instance: String,
    pub folder: PathBuf,
    pub loader: String,
    pub minecraft: String,
}

/// What a check found: the records of the jars the platforms named, the newer release of each checked mod that has one,
/// and the file names of the mods checked. A checked mod with no entry in `updates` has no newer release.
#[derive(Debug, Clone, Default)]
pub struct Checks {
    pub identified: Vec<ModSource>,
    pub updates: HashMap<String, ContentVersion>,
    pub checked: Vec<String>,
}

/// Checks an instance's mods against their platforms. `unknown` names the jars to look up by their hash, and only those
/// with no record are looked up. `records` names the mods to check for a newer release, and `None` checks every record
/// the instance holds, the ones just identified included. The records found are saved, and each hash computed is cached.
pub async fn check_mods(db: Database, context: CheckContext, unknown: Vec<String>, records: Option<Vec<ModSource>>) -> Result<Checks, String> {
    let recorded: HashMap<String, ModSource> = db
        .mod_sources_of(&context.instance)
        .await?
        .into_iter()
        .map(|record| (record.file_name.clone(), record))
        .collect();
    let cached: HashMap<String, JarHash> = db
        .jar_hashes(&context.instance)
        .await?
        .into_iter()
        .map(|hash| (hash.file_name.clone(), hash))
        .collect();
    let mods_dir = context.folder.join("mods");

    // The jars with no record are hashed. Only a jar hashed just now is looked up on the platform: a cached hash was
    // looked up when it was computed, so a jar nobody has is not asked about again on every open.
    let lookups: Vec<String> = unknown.into_iter().filter(|name| !recorded.contains_key(name)).collect();
    let (hashed, fresh) = hash_in_background(mods_dir.clone(), lookups, cached.clone()).await?;
    let new_names: HashSet<&str> = fresh.iter().map(|hash| hash.file_name.as_str()).collect();
    let to_identify: Vec<(String, String)> = hashed.iter().filter(|(name, _)| new_names.contains(name.as_str())).cloned().collect();
    let identified = identify(&context.instance, &to_identify).await?;
    db.record_mod_sources(&identified).await?;
    db.record_jar_hashes(&context.instance, &fresh).await?;

    // The records to check: the ones named, or every record the instance holds once the new ones are in.
    let mut checking: Vec<ModSource> = match records {
        Some(records) => records,
        None => recorded.into_values().chain(identified.iter().cloned()).collect(),
    };
    let missing: Vec<String> = checking
        .iter()
        .filter(|record| record.sha1.is_none())
        .map(|record| record.file_name.clone())
        .collect();
    if !missing.is_empty() {
        let (more, fresh) = hash_in_background(mods_dir, missing, cached).await?;
        let more: HashMap<String, String> = more.into_iter().collect();
        for record in &mut checking {
            if record.sha1.is_none() {
                record.sha1 = more.get(&record.file_name).cloned();
            }
        }
        db.record_jar_hashes(&context.instance, &fresh).await?;
    }

    let (updates, checked) = newer_releases(&context, &checking).await?;
    Ok(Checks {
        identified,
        updates,
        checked,
    })
}

/// The SHA-1 of each jar named that the folder holds, with the hashes computed fresh, which the cache keeps. The hash
/// of a jar whose size and modification time match its cached hash is reused, and the rest are read. Runs on the
/// blocking pool, since reading a jar takes time.
async fn hash_in_background(
    mods_dir: PathBuf,
    names: Vec<String>,
    cached: HashMap<String, JarHash>,
) -> Result<(Vec<(String, String)>, Vec<JarHash>), String> {
    tokio::task::spawn_blocking(move || hash_jars(&mods_dir, &names, &cached))
        .await
        .map_err(|error| error.to_string())
}

fn hash_jars(mods_dir: &Path, names: &[String], cached: &HashMap<String, JarHash>) -> (Vec<(String, String)>, Vec<JarHash>) {
    let mut hashes = Vec::with_capacity(names.len());
    let mut fresh = Vec::new();
    for name in names {
        let path = mods_dir.join(name);
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        let size = meta.len();
        let modified_at = meta
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |since| since.as_secs() as i64);
        match cached.get(name).filter(|hash| hash.size == size && hash.modified_at == modified_at) {
            Some(hash) => hashes.push((name.clone(), hash.sha1.clone())),
            None => match sha1_of_file(&path) {
                Ok(sha1) => {
                    hashes.push((name.clone(), sha1.clone()));
                    fresh.push(JarHash {
                        file_name: name.clone(),
                        size,
                        modified_at,
                        sha1,
                    });
                }
                Err(error) => warn!("hashing {} failed: {error}", path.display()),
            },
        }
    }
    (hashes, fresh)
}

/// The records of the jars whose hash a Modrinth release has: each names its project and release. A jar with no release
/// gets no record, and nor does one whose release names no project. Names come from one batched call.
async fn identify(instance: &str, hashed: &[(String, String)]) -> Result<Vec<ModSource>, String> {
    if hashed.is_empty() {
        return Ok(Vec::new());
    }
    let hashes: Vec<String> = hashed.iter().map(|(_, sha1)| sha1.clone()).collect();
    let releases = MODRINTH.versions_by_hashes(&hashes).await.map_err(|error| error.to_string())?;
    let mut identified: Vec<ModSource> = hashed
        .iter()
        .filter_map(|(file_name, sha1)| {
            let release = releases.get(sha1)?;
            Some(ModSource {
                instance: instance.to_string(),
                platform: Source::Modrinth.key().to_string(),
                project_id: release.project_id.clone()?,
                version_id: release.id.clone(),
                name: stem(file_name),
                file_name: file_name.clone(),
                sha1: Some(sha1.clone()),
            })
        })
        .collect();
    if identified.is_empty() {
        return Ok(identified);
    }
    let project_ids: Vec<String> = identified
        .iter()
        .map(|record| record.project_id.clone())
        .collect::<HashSet<String>>()
        .into_iter()
        .collect();
    let names = match MODRINTH.project_names(&project_ids).await {
        Ok(names) => names,
        Err(error) => {
            warn!("reading the names of the identified mods failed: {error}");
            Default::default()
        }
    };
    for record in &mut identified {
        if let Some(name) = names.get(&record.project_id) {
            record.name = name.clone();
        }
    }
    Ok(identified)
}

/// The newer release of each checked mod that has one, and the file names of the mods checked. A Modrinth mod is checked
/// by its hash, all at once. A CurseForge mod is checked by listing its project's files, a few projects at a time. A mod
/// with no hash, or whose platform gave no answer, is not checked, so its update stands as it was.
async fn newer_releases(context: &CheckContext, records: &[ModSource]) -> Result<(HashMap<String, ContentVersion>, Vec<String>), String> {
    let mut updates = HashMap::new();
    let mut checked = Vec::new();

    let modrinth: Vec<&ModSource> = records
        .iter()
        .filter(|record| record.platform == Source::Modrinth.key() && record.sha1.is_some())
        .collect();
    if !modrinth.is_empty() {
        let hashes: Vec<String> = modrinth.iter().filter_map(|record| record.sha1.clone()).collect();
        let latest = MODRINTH
            .latest_by_hashes(&hashes, std::slice::from_ref(&context.loader), std::slice::from_ref(&context.minecraft))
            .await
            .map_err(|error| error.to_string())?;
        for record in modrinth {
            let Some(sha1) = record.sha1.as_deref() else {
                continue;
            };
            checked.push(record.file_name.clone());
            if let Some(release) = latest.get(sha1).filter(|release| release.id != record.version_id) {
                updates.insert(record.file_name.clone(), release.clone());
            }
        }
    }

    let curseforge_records: Vec<ModSource> = records
        .iter()
        .filter(|record| record.platform == Source::CurseForge.key())
        .cloned()
        .collect();
    // Without an API key CurseForge cannot be asked, so its mods are left as they are.
    let platform = curseforge_records.first().and_then(|_| curseforge().ok());
    if let Some(platform) = platform {
        for batch in curseforge_records.chunks(CURSEFORGE_AT_ONCE) {
            let mut calls = JoinSet::new();
            for record in batch.iter().cloned() {
                let (loader, minecraft) = (context.loader.clone(), context.minecraft.clone());
                calls.spawn(async move {
                    let releases = platform.fitting_files(&record.project_id, &minecraft, &loader).await;
                    (record, releases)
                });
            }
            while let Some(joined) = calls.join_next().await {
                let (record, releases) = joined.map_err(|error| error.to_string())?;
                let releases = match releases {
                    Ok(releases) => releases,
                    Err(error) => {
                        warn!("checking {} on CurseForge failed: {error}", record.name);
                        continue;
                    }
                };
                checked.push(record.file_name.clone());
                if let Some(release) = fitting(releases, &context.loader, &context.minecraft).filter(|release| release.id != record.version_id) {
                    updates.insert(record.file_name.clone(), release);
                }
            }
        }
    }
    Ok((updates, checked))
}

/// The SHA-1 of the file at `path`, as lowercase hex.
pub(crate) fn sha1_of_file(path: &Path) -> Result<String, String> {
    let _scope = perf::scope("io.hash");
    sha1_of_reader(std::fs::File::open(path).map_err(|error| error.to_string())?)
}

/// The SHA-1 of everything `reader` yields, as lowercase hex.
pub(crate) fn sha1_of_reader(mut reader: impl Read) -> Result<String, String> {
    let mut hasher = Sha1::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
#[path = "../../tests/app/mod_sources.rs"]
mod tests;
