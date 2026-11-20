//! Snapshot files: where they live, and how they are written, listed, read, pruned and restored.
//!
//! A snapshot is two files beside each other: its JSON, compressed with xz, and, when it holds local jars, a zip of those
//! jars. The zip is written first and the JSON last, each through a `.partial` file that is renamed once whole, so a snapshot
//! counts only once its JSON exists.

use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;
use sha1::{Digest, Sha1};
use xz2::read::XzDecoder;
use xz2::write::XzEncoder;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use crate::error::{Error, Result};
use crate::hash::{hex, sha1_of_file};
use crate::model::{Kind, LocalJar, Snapshot};
use crate::naming::{self, JSON_EXTENSION, Name, ZIP_EXTENSION};

/// The xz preset snapshots are written at.
const PRESET: u32 = 9;
/// The bytes a copy reads at a time.
const CHUNK: usize = 64 * 1024;

/// The folder that holds an instance's snapshots.
pub fn dir(instance: &Path) -> PathBuf {
    instance.join(".lodestone").join("snapshots")
}

/// A jar a snapshot keeps: its file name in `mods/`, and where it is now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalFile {
    pub file: String,
    pub path: PathBuf,
}

/// A snapshot on disk: its name, its JSON, and its zip of local jars when it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub name: Name,
    pub json: PathBuf,
    pub zip: Option<PathBuf>,
}

impl Listed {
    /// The snapshot's files, the JSON first.
    pub fn files(&self) -> Vec<&Path> {
        std::iter::once(self.json.as_path()).chain(self.zip.as_deref()).collect()
    }
}

/// The snapshots in `dir`, newest first. A folder that does not exist holds none. A zip with no JSON beside it, and any file
/// that is not a snapshot's, is left out.
pub fn list(dir: &Path) -> Result<Vec<Listed>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut jsons: Vec<Listed> = Vec::new();
    let mut zips: Vec<(Name, PathBuf)> = Vec::new();
    for entry in entries {
        let path = entry?.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Some(name) = naming::parse_file(file_name) else {
            continue;
        };
        if file_name.ends_with(JSON_EXTENSION) {
            jsons.push(Listed { name, json: path, zip: None });
        } else {
            zips.push((name, path));
        }
    }
    for listed in &mut jsons {
        listed.zip = zips.iter().find(|(name, _)| *name == listed.name).map(|(_, path)| path.clone());
    }
    jsons.sort_by_key(|listed| std::cmp::Reverse(listed.name));
    Ok(jsons)
}

/// Reads the snapshot whose JSON is at `json`.
pub fn read(json: &Path) -> Result<Snapshot> {
    let decoder = XzDecoder::new(BufReader::new(File::open(json)?));
    Ok(serde_json::from_reader(decoder)?)
}

/// Writes a snapshot of `kind`, taken at `when`, into `dir`, and returns the files it made.
///
/// `snapshot` holds the platform mods. Each of `local` is hashed and kept in a zip beside the JSON, and the JSON's `local`
/// section lists it with that hash, replacing any section `snapshot` came with. The name takes the first copy number the
/// second has free, so no snapshot is overwritten.
pub fn write(dir: &Path, kind: Kind, when: NaiveDateTime, mut snapshot: Snapshot, local: &[LocalFile]) -> Result<Listed> {
    fs::create_dir_all(dir)?;
    snapshot.local = local
        .iter()
        .map(|file| {
            Ok(LocalJar {
                file: file.file.clone(),
                sha1: sha1_of_file(&file.path)?,
            })
        })
        .collect::<io::Result<Vec<LocalJar>>>()?;
    let name = Name {
        when,
        copy: free_copy(dir, when, kind),
        kind,
    };
    let stem = name.stem();
    let json = dir.join(format!("{stem}.{JSON_EXTENSION}"));
    let zip = (!local.is_empty()).then(|| dir.join(format!("{stem}.{ZIP_EXTENSION}")));
    if let Some(zip_path) = &zip {
        write_zip(zip_path, local)?;
    }
    if let Err(error) = write_json(&json, &snapshot) {
        if let Some(zip_path) = &zip {
            let _ = fs::remove_file(zip_path);
        }
        return Err(error);
    }
    Ok(Listed { name, json, zip })
}

/// Removes the snapshots in `dir` beyond the newest `keep`, and returns the ones it removed.
pub fn prune(dir: &Path, keep: u32) -> Result<Vec<Listed>> {
    let mut removed = Vec::new();
    for old in list(dir)?.into_iter().skip(keep as usize) {
        remove(&old)?;
        removed.push(old);
    }
    Ok(removed)
}

/// Removes a snapshot's files, the JSON first, so a snapshot that is only partly removed is not listed.
pub fn remove(listed: &Listed) -> io::Result<()> {
    fs::remove_file(&listed.json)?;
    if let Some(zip) = &listed.zip
        && let Err(error) = fs::remove_file(zip)
        && error.kind() != io::ErrorKind::NotFound
    {
        return Err(error);
    }
    Ok(())
}

/// Copies the jar `file` out of a snapshot's zip to `dest`. The bytes are hashed as they are copied, and `dest` is put in
/// place only once they match `sha1`, so a damaged zip leaves no jar behind. `dest` must not exist yet.
pub fn restore_local(zip: &Path, file: &str, sha1: &str, dest: &Path) -> Result<()> {
    if dest.exists() {
        return Err(Error::Exists(dest.to_path_buf()));
    }
    let partial = partial_of(dest);
    let result = (|| -> Result<()> {
        let mut archive = zip::ZipArchive::new(BufReader::new(File::open(zip)?))?;
        let mut entry = archive.by_name(file)?;
        let mut out = BufWriter::new(File::create(&partial)?);
        let mut hasher = Sha1::new();
        let mut buffer = vec![0; CHUNK];
        loop {
            let read = entry.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            out.write_all(&buffer[..read])?;
        }
        out.flush()?;
        out.into_inner().map_err(|error| error.into_error())?.sync_all()?;
        if !hex(&hasher.finalize()).eq_ignore_ascii_case(sha1) {
            return Err(Error::Checksum { file: file.to_string() });
        }
        fs::rename(&partial, dest)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result
}

/// The first copy number for `kind` at `when` that has no file or partial file in `dir`.
fn free_copy(dir: &Path, when: NaiveDateTime, kind: Kind) -> u32 {
    let mut copy = 1;
    while [JSON_EXTENSION, ZIP_EXTENSION].iter().any(|extension| {
        let file = format!("{}.{extension}", naming::stem(when, kind, copy));
        dir.join(&file).exists() || dir.join(format!("{file}.partial")).exists()
    }) {
        copy += 1;
    }
    copy
}

/// The name a file is written under until it is whole: its own name with `.partial` added.
fn partial_of(path: &Path) -> PathBuf {
    let mut partial = path.as_os_str().to_owned();
    partial.push(".partial");
    PathBuf::from(partial)
}

/// Writes the zip of local jars at `path`, each jar stored as it is, since jars are compressed already.
fn write_zip(path: &Path, local: &[LocalFile]) -> Result<()> {
    let partial = partial_of(path);
    let result = (|| -> Result<()> {
        let mut zip = zip::ZipWriter::new(BufWriter::new(File::create(&partial)?));
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        for file in local {
            zip.start_file(file.file.as_str(), options)?;
            io::copy(&mut File::open(&file.path)?, &mut zip)?;
        }
        let mut out = zip.finish()?;
        out.flush()?;
        out.into_inner().map_err(|error| error.into_error())?.sync_all()?;
        fs::rename(&partial, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result
}

/// Writes the snapshot's JSON, compressed with xz, at `path`.
fn write_json(path: &Path, snapshot: &Snapshot) -> Result<()> {
    let partial = partial_of(path);
    let result = (|| -> Result<()> {
        let mut encoder = XzEncoder::new(BufWriter::new(File::create(&partial)?), PRESET);
        serde_json::to_writer_pretty(&mut encoder, snapshot)?;
        let mut out = encoder.finish()?;
        out.flush()?;
        out.into_inner().map_err(|error| error.into_error())?.sync_all()?;
        fs::rename(&partial, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result
}

#[cfg(test)]
#[path = "../tests/store.rs"]
mod tests;
