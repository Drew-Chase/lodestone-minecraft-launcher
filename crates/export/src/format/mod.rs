//! The package writers, and what they share: the formats, the metadata a package says about its instance,
//! and the zip helpers.

pub mod curseforge;
pub mod lodestone;
pub mod mrpack;
pub mod plain;

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::error::Result;
use crate::link::Entry;
use crate::progress::Control;

/// The formats an export can write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// A Modrinth pack, `.mrpack`.
    Mrpack,
    /// A CurseForge pack.
    CurseForge,
    /// A zip of the selected files.
    Zip,
    /// A Lodestone package, `.lodestone`.
    Lodestone,
    /// A portable launcher: a copy of the launcher with a Lodestone package after it.
    Portable,
}

/// What a package says about its instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    /// The name the package is known by.
    pub name: String,
    /// The package's own version, such as `1.0.0`.
    pub version: String,
    /// What the package is for. Empty when the author gave none.
    pub description: String,
    /// The Minecraft version, such as `1.21.1`.
    pub minecraft: String,
    /// The loader's key: `vanilla`, `fabric`, `quilt`, `forge` or `neoforge`.
    pub loader: String,
    /// The loader's own version, for a modded instance.
    pub loader_version: Option<String>,
}

/// What a Lodestone package or a portable launcher holds: the identity of the package and the files it takes.
pub struct Package<'a> {
    /// Random per export. A portable launcher keeps its folders under it, so two exports never share one.
    pub id: String,
    /// When the package was written, in seconds since the Unix epoch.
    pub exported_at: u64,
    pub metadata: &'a Metadata,
    pub entries: &'a [Entry],
    /// Whether linked files keep their bytes in the package. A plain package bundles only the files with no
    /// link. An offline portable launcher bundles every file.
    pub bundle_linked: bool,
}

/// The options a file is stored with in a zip. Jars and zips are already compressed, so they are stored as
/// they are. The file keeps its permission bits, so an executable stays executable.
pub(crate) fn file_options(name: &str, source: &Path) -> SimpleFileOptions {
    let stored = matches!(name.rsplit('.').next(), Some("jar" | "zip"));
    let method = if stored {
        CompressionMethod::Stored
    } else {
        CompressionMethod::Deflated
    };
    let options = SimpleFileOptions::default().compression_method(method);
    match permissions(source) {
        Some(mode) => options.unix_permissions(mode),
        None => options,
    }
}

#[cfg(unix)]
fn permissions(source: &Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(source).ok().map(|meta| meta.permissions().mode() & 0o777)
}

#[cfg(not(unix))]
fn permissions(_source: &Path) -> Option<u32> {
    None
}

/// Whether `format` writes the bytes of `entry` into the package. A linked file is not written unless
/// `bundle_linked` is set, and a file the format cannot express as a link is always written.
pub fn is_bundled(format: Format, entry: &Entry, bundle_linked: bool) -> bool {
    match format {
        Format::Mrpack => !mrpack::is_linked(entry),
        Format::CurseForge => !curseforge::is_linked(entry),
        Format::Zip => true,
        Format::Lodestone | Format::Portable => bundle_linked || !entry.is_linked(),
    }
}

/// Writes a zip to `target`: `manifest` as `(name, bytes)` at the root, then each of `files` under `prefix`.
pub(crate) fn write_archive(target: &Path, manifest: (&str, &[u8]), files: &[&Entry], prefix: &str, control: &mut Control<'_>) -> Result<()> {
    let mut zip = ZipWriter::new(BufWriter::new(File::create(target)?));
    let json = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file(manifest.0, json)?;
    zip.write_all(manifest.1)?;
    for entry in files {
        control.check()?;
        let name = format!("{prefix}{}", entry.path);
        zip.start_file(&name, file_options(&name, &entry.source))?;
        std::io::copy(&mut File::open(&entry.source)?, &mut zip)?;
        control.advance(entry.size)?;
    }
    zip.finish()?.flush()?;
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/format/mod.rs"]
mod tests;
