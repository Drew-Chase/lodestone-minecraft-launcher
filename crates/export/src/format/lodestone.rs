//! The Lodestone package, `.lodestone`: a tar inside an xz container at preset 9. `lodestone.json` lists every
//! mod and resource file with its digests and any links, and `overrides/` holds the bytes the package bundles.
//! The reader takes the same layout back, so a portable launcher can install from it.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use xz2::read::XzDecoder;
use xz2::write::XzEncoder;

use super::Package;
use crate::error::{Error, Result};
use crate::link::Links;
use crate::progress::Control;

/// The manifest's name inside the package.
pub const MANIFEST: &str = "lodestone.json";
/// The folder inside the package that holds the bundled bytes.
pub const OVERRIDES: &str = "overrides";
/// The layout version. A reader refuses a package with a newer one.
pub const FORMAT: u32 = 1;
/// The xz preset the package is written at.
const PRESET: u32 = 9;

/// The package's manifest, `lodestone.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// The layout version.
    pub format: u32,
    /// The package's id, from [`Package::id`].
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// When the package was written, in seconds since the Unix epoch.
    pub exported_at: u64,
    pub minecraft: String,
    pub loader: LoaderRecord,
    /// The mod and resource files, with their digests and links.
    #[serde(default)]
    pub files: Vec<FileRecord>,
    /// The paths inside `overrides/`, for the bundled files. Empty for a plain zip, whose files sit at their paths.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<String>,
}

/// The loader the instance runs on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoaderRecord {
    /// The loader's key: `vanilla`, `fabric`, `quilt`, `forge` or `neoforge`.
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// One mod or resource file: its path, its digests, whether its bytes are in the package, and its links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRecord {
    pub path: String,
    pub sha1: String,
    pub sha512: String,
    pub size: u64,
    pub bundled: bool,
    #[serde(default, skip_serializing_if = "Links::is_empty")]
    pub links: Links,
}

/// The manifest for `package`. A file's bytes are bundled when `bundle_linked` is set or the file has no link.
pub fn manifest(package: &Package<'_>, bundle_linked: bool) -> Manifest {
    let mut files = Vec::new();
    let mut overrides = Vec::new();
    for entry in package.entries {
        let bundled = bundle_linked || !entry.is_linked();
        if bundled {
            overrides.push(entry.path.clone());
        }
        if let Some(digests) = &entry.digests {
            files.push(FileRecord {
                path: entry.path.clone(),
                sha1: digests.sha1.clone(),
                sha512: digests.sha512.clone(),
                size: entry.size,
                bundled,
                links: entry.links.clone(),
            });
        }
    }
    Manifest {
        format: FORMAT,
        id: package.id.clone(),
        name: package.metadata.name.clone(),
        version: package.metadata.version.clone(),
        description: package.metadata.description.clone(),
        exported_at: package.exported_at,
        minecraft: package.metadata.minecraft.clone(),
        loader: LoaderRecord {
            id: package.metadata.loader.clone(),
            version: package.metadata.loader_version.clone(),
        },
        files,
        overrides,
    }
}

/// Writes `package` as a Lodestone package into `sink`, and returns the sink.
pub fn write_to<W: Write>(sink: W, package: &Package<'_>, control: &mut Control<'_>) -> Result<W> {
    let manifest = manifest(package, package.bundle_linked);
    let json = serde_json::to_vec_pretty(&manifest)?;
    let mut encoder = XzEncoder::new(sink, PRESET);
    {
        let mut tar = tar::Builder::new(&mut encoder);
        append_bytes(&mut tar, MANIFEST, &json, package.exported_at)?;
        for entry in package.entries.iter().filter(|entry| package.bundle_linked || !entry.is_linked()) {
            control.check()?;
            let mut file = File::open(&entry.source)?;
            tar.append_file(format!("{OVERRIDES}/{}", entry.path), &mut file)?;
            control.advance(entry.size)?;
        }
        tar.finish()?;
    }
    Ok(encoder.finish()?)
}

/// Adds `bytes` to `tar` as a regular file at `path`.
fn append_bytes<W: Write>(tar: &mut tar::Builder<W>, path: &str, bytes: &[u8], modified: u64) -> io::Result<()> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_mtime(modified);
    header.set_entry_type(tar::EntryType::Regular);
    tar.append_data(&mut header, path, bytes)
}

/// Reads the manifest out of a Lodestone package. The manifest is the package's first entry, so only the start
/// of the stream is decoded.
pub fn read_manifest(source: impl Read) -> Result<Manifest> {
    let (counted, consumed) = lodestone_fs::Counted::new(source);
    let mut archive = tar::Archive::new(lodestone_fs::Bounded::new(XzDecoder::new(counted), consumed));
    for entry in archive.entries()? {
        let mut entry = entry?;
        if entry.path()?.as_ref() == Path::new(MANIFEST) {
            let manifest: Manifest = serde_json::from_reader(&mut entry)?;
            if manifest.format > FORMAT {
                return Err(Error::Invalid(format!(
                    "this package needs a newer launcher (layout {})",
                    manifest.format
                )));
            }
            return Ok(manifest);
        }
    }
    Err(Error::Invalid(format!("the package has no {MANIFEST}")))
}

/// Unpacks the bundled files of a Lodestone package into `destination`, at the paths they have inside
/// `overrides/`. A path that would leave `destination` is refused.
pub fn extract_overrides(source: impl Read, destination: &Path) -> Result<()> {
    let (counted, consumed) = lodestone_fs::Counted::new(source);
    let mut archive = tar::Archive::new(lodestone_fs::Bounded::new(XzDecoder::new(counted), consumed));
    for entry in archive.entries()? {
        let mut entry = entry?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let path = entry.path()?.into_owned();
        let Ok(relative) = path.strip_prefix(OVERRIDES) else {
            continue;
        };
        if relative.as_os_str().is_empty() {
            continue;
        }
        if !relative.components().all(|part| matches!(part, Component::Normal(_))) {
            return Err(Error::Invalid(format!(
                "the package names a path outside the instance: {}",
                path.display()
            )));
        }
        let target = destination.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        entry.unpack(&target)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/format/lodestone.rs"]
mod tests;
