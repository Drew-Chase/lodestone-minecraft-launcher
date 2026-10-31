//! Technic zip packs: a zip of the whole instance. The loader is named by
//! `bin/version.json`, or by the same file inside `bin/modpack.jar`. The install
//! unpacks everything except those two descriptors.

use std::io::{Cursor, Read, Seek};
use std::path::Path;

use serde::Deserialize;

use crate::error::PlatformError;
use crate::meta::Modpack;

/// The loader descriptors. The install leaves them out of the unpacked instance.
const DESCRIPTORS: [&str; 2] = ["bin/modpack.jar", "bin/version.json"];

/// The fields of a `version.json` that name the loader.
#[derive(Debug, Deserialize)]
struct VersionJson {
    id: String,
    #[serde(default)]
    libraries: Vec<Library>,
}

#[derive(Debug, Deserialize)]
struct Library {
    name: String,
}

/// What a descriptor names: the loader, its version, and the Minecraft version it targets.
#[derive(Debug, Default)]
struct Described {
    loader: Option<&'static str>,
    loader_version: Option<String>,
    minecraft: Option<String>,
}

/// Reads a Technic zip into the modpack it describes. The zip unpacks whole, except for the loader
/// descriptors. The Minecraft version comes from the Forge version when the zip names one, and is
/// empty otherwise, so the caller supplies it.
pub(crate) fn parse(archive: &Path) -> Result<Modpack, PlatformError> {
    let file = std::fs::File::open(archive).map_err(|source| PlatformError::Io {
        path: archive.to_path_buf(),
        source,
    })?;
    read_pack(file, archive)
}

/// The modpack a Technic zip describes, read from any seekable reader. `archive` names the pack, and
/// the file in errors.
fn read_pack<R: Read + Seek>(reader: R, archive: &Path) -> Result<Modpack, PlatformError> {
    let mut zip = zip::ZipArchive::new(reader).map_err(|error| parse_error(archive, error))?;
    let descriptor = match read_entry(&mut zip, archive, "bin/version.json")? {
        Some(bytes) => Some(bytes),
        None => read_nested_descriptor(&mut zip, archive)?,
    };
    let described = match descriptor {
        Some(bytes) => describe(&bytes, archive)?,
        None => Described::default(),
    };
    Ok(Modpack {
        name: archive.file_stem().map_or_else(String::new, |stem| stem.to_string_lossy().into_owned()),
        version: None,
        minecraft: described.minecraft.unwrap_or_default(),
        loader: described.loader,
        loader_version: described.loader_version,
        files: Vec::new(),
        overrides: Some(String::new()),
        overrides_skip: DESCRIPTORS.iter().map(|name| name.to_string()).collect(),
        archives: Vec::new(),
    })
}

/// The `version.json` inside `bin/modpack.jar`, when the pack ships its descriptor there.
fn read_nested_descriptor<R: Read + Seek>(zip: &mut zip::ZipArchive<R>, archive: &Path) -> Result<Option<Vec<u8>>, PlatformError> {
    let Some(jar) = read_entry(zip, archive, "bin/modpack.jar")? else {
        return Ok(None);
    };
    let mut nested = zip::ZipArchive::new(Cursor::new(jar)).map_err(|error| parse_error(archive, error))?;
    read_entry(&mut nested, archive, "version.json")
}

/// One entry's bytes, or `None` when the archive has no such entry.
fn read_entry<R: Read + Seek>(zip: &mut zip::ZipArchive<R>, archive: &Path, name: &str) -> Result<Option<Vec<u8>>, PlatformError> {
    match zip.by_name(name) {
        Ok(mut entry) => {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).map_err(|source| PlatformError::Io {
                path: archive.to_path_buf(),
                source,
            })?;
            Ok(Some(bytes))
        }
        Err(zip::result::ZipError::FileNotFound) => Ok(None),
        Err(error) => Err(parse_error(archive, error)),
    }
}

fn parse_error(archive: &Path, error: zip::result::ZipError) -> PlatformError {
    PlatformError::ModpackParse {
        path: archive.to_path_buf(),
        source: Box::new(error),
    }
}

/// The loader, its version and the Minecraft version a `version.json` names. The libraries decide the
/// loader; a pack with none of the known loader libraries is vanilla.
fn describe(bytes: &[u8], archive: &Path) -> Result<Described, PlatformError> {
    let descriptor: VersionJson = serde_json::from_slice(bytes).map_err(|error| PlatformError::ModpackParse {
        path: archive.to_path_buf(),
        source: Box::new(error),
    })?;
    let library = |prefixes: &[&str]| {
        descriptor
            .libraries
            .iter()
            .find_map(|library| prefixes.iter().find_map(|prefix| library.name.strip_prefix(prefix)))
            .map(str::to_string)
    };
    if let Some(version) = library(&[
        "net.minecraftforge:forge:",
        "net.minecraftforge:minecraftforge:",
        "net.minecraftforge:fmlloader:",
    ]) {
        // Forge's version carries its Minecraft version in front: `1.12.2-14.23.5.2847`.
        let minecraft = version.split_once('-').map(|(minecraft, _)| minecraft.to_string());
        return Ok(Described {
            loader: Some("forge"),
            loader_version: Some(version),
            minecraft,
        });
    }
    if let Some(version) = library(&["net.fabricmc:fabric-loader:"]) {
        return Ok(Described {
            loader: Some("fabric"),
            loader_version: Some(version),
            minecraft: None,
        });
    }
    if let Some(version) = library(&["org.quiltmc:quilt-loader:"]) {
        return Ok(Described {
            loader: Some("quilt"),
            loader_version: Some(version),
            minecraft: None,
        });
    }
    if descriptor.id.starts_with("neoforge-") {
        return Ok(Described {
            loader: Some("neoforge"),
            loader_version: library(&["net.neoforged:neoforge:"]),
            minecraft: None,
        });
    }
    Ok(Described::default())
}

#[cfg(test)]
#[path = "../../tests/modpack/technic.rs"]
mod tests;
