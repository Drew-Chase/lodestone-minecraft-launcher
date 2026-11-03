//! The background checks that raise notifications: a newer release of an instance's modpack, and newer releases of its mods.
//! They run as the library loads, and then every few hours. What they find is written straight to the notifications, so it
//! shows at the next read.

use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Local;
use lodestone_platforms::{ContentType, ContentVersion};
use tracing::warn;

use super::{Draft, Kind};
use crate::app::database::Database;
use crate::app::discover::versions_of;
use crate::app::mod_sources::{CheckContext, check_mods};
use crate::app::pages::discover::Source;

/// How long after a check the next one may start.
pub const EVERY: Duration = Duration::from_secs(6 * 60 * 60);

/// An installed instance, as a check reads it.
#[derive(Debug, Clone)]
pub struct Target {
    pub id: String,
    pub name: String,
    pub mc_version: String,
    pub loader: String,
    /// The instance's folder, which holds its `mods` folder.
    pub folder: PathBuf,
    /// Where the instance's modpack came from, when it came from a platform.
    pub pack: Option<Pack>,
}

/// A modpack on a platform, and the release the instance was installed with.
#[derive(Debug, Clone)]
pub struct Pack {
    pub platform: String,
    pub project_id: String,
    pub installed: Option<String>,
}

/// One release of a modpack: its version, and the Minecraft versions it targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub minecraft: Vec<String>,
}

impl From<&ContentVersion> for Release {
    fn from(release: &ContentVersion) -> Self {
        Self {
            version: release.version_number.clone(),
            minecraft: release.minecraft_versions.clone(),
        }
    }
}

/// What a modpack's releases mean for an instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackNews {
    /// There is nothing new for the instance.
    Nothing,
    /// The instance has no recorded release, so this one becomes its baseline.
    Baseline(String),
    /// A newer release for the instance's Minecraft version is out.
    Update(String),
}

/// The news from `releases`, which the platform lists newest first, for an instance at `mc_version` that was installed with
/// `installed`. Only a release that targets the instance's Minecraft version counts.
pub fn pack_news(installed: Option<&str>, releases: &[Release], mc_version: &str) -> PackNews {
    let newest = releases
        .iter()
        .find(|release| release.minecraft.iter().any(|version| version == mc_version));
    match (newest, installed) {
        (None, _) => PackNews::Nothing,
        (Some(release), None) => PackNews::Baseline(release.version.clone()),
        (Some(release), Some(installed)) if release.version == installed => PackNews::Nothing,
        (Some(release), Some(_)) => PackNews::Update(release.version.clone()),
    }
}

/// The names of the jar files in a `mods` folder.
fn jar_names(mods: &Path) -> Vec<String> {
    std::fs::read_dir(mods)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|extension| extension == "jar"))
                .filter_map(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
                .collect()
        })
        .unwrap_or_default()
}

/// Checks each instance in turn. An instance that cannot be checked is logged and left out, and the rest still run.
pub async fn run(db: Database, targets: Vec<Target>) {
    for target in &targets {
        if let Some(pack) = &target.pack {
            check_pack(&db, target, pack).await;
        }
        check_mods_of(&db, target).await;
    }
}

/// Checks an instance's modpack against its platform, and records a newer release, or the baseline when there is none.
async fn check_pack(db: &Database, target: &Target, pack: &Pack) {
    let Some(source) = Source::from_key(&pack.platform) else {
        return;
    };
    let releases: Vec<Release> = match versions_of(source, ContentType::Modpack, &pack.project_id).await {
        Ok(releases) => releases.iter().map(Release::from).collect(),
        Err(error) => {
            warn!("Could not check the modpack of {}: {error}", target.name);
            return;
        }
    };
    let recorded = match pack_news(pack.installed.as_deref(), &releases, &target.mc_version) {
        PackNews::Nothing => return,
        PackNews::Baseline(version) => db.set_source_version(&target.id, &version).await,
        PackNews::Update(version) => {
            let draft = Draft {
                key: format!("pack:{}:{version}", target.id),
                kind: Kind::ModpackUpdate,
                subject: target.name.clone(),
                detail: version,
                instance_id: Some(target.id.clone()),
                target: Some(format!("{}/{}", pack.platform, pack.project_id)),
                created_at: Local::now(),
            };
            db.record_notification(&draft).await
        }
    };
    if let Err(error) = recorded {
        warn!("Could not record the modpack check of {}: {error}", target.name);
    }
}

/// Checks an instance's mods against their platforms, and records how many have a newer release.
async fn check_mods_of(db: &Database, target: &Target) {
    let names = jar_names(&target.folder.join("mods"));
    if names.is_empty() {
        return;
    }
    let context = CheckContext {
        instance: target.id.clone(),
        folder: target.folder.clone(),
        loader: target.loader.clone(),
        minecraft: target.mc_version.clone(),
    };
    let checks = match check_mods(db.clone(), context, names, None).await {
        Ok(checks) => checks,
        Err(error) => {
            warn!("Could not check the mods of {}: {error}", target.name);
            return;
        }
    };
    let count = checks.updates.len();
    if count == 0 {
        return;
    }
    let draft = Draft {
        key: format!("mods:{}:{count}", target.id),
        kind: Kind::ModUpdate,
        subject: target.name.clone(),
        detail: count.to_string(),
        instance_id: Some(target.id.clone()),
        target: None,
        created_at: Local::now(),
    };
    if let Err(error) = db.record_notification(&draft).await {
        warn!("Could not record the mod check of {}: {error}", target.name);
    }
}

#[cfg(test)]
#[path = "../../../tests/app/notifications/checks.rs"]
mod tests;
