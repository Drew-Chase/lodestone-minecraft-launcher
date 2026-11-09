//! Writes the server pack, as a folder of its files or a zip of them. The pack is written to a partial folder or file
//! beside the target and renamed into place once it is whole, so a cancel or a failure leaves nothing behind. A target
//! that already exists is refused rather than replaced, and so is one inside the instance it reads.

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use crate::error::{Error, Result};
use crate::format::file_options;
use crate::plan::Planned;
use crate::progress::{Control, Progress};

/// Where the pack is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    /// A new folder that holds the files at their paths.
    Folder(PathBuf),
    /// A new zip file that holds the files at their paths.
    Zip(PathBuf),
}

impl Output {
    /// The path the pack is written to.
    pub fn target(&self) -> &Path {
        match self {
            Output::Folder(target) | Output::Zip(target) => target,
        }
    }
}

/// Writes `files` to `output`, reporting after each file, and returns the path written. Fails when the target already
/// exists or lies inside `instance`, and when `cancel` is set between files. A failed or cancelled write leaves no
/// partial output behind.
pub fn write(files: &[Planned], output: &Output, instance: &Path, cancel: &AtomicBool, report: &mut dyn FnMut(Progress)) -> Result<PathBuf> {
    let target = output.target().to_path_buf();
    if target.starts_with(instance) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "the pack cannot be written inside its instance").into());
    }
    if target.exists() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("{} already exists", target.display())).into());
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let partial = partial_path(&target);
    remove_partial(&partial)?;
    let total_bytes = files.iter().map(|file| file.size).sum();
    let written = match output {
        Output::Folder(_) => write_folder(files, &partial, total_bytes, cancel, report),
        Output::Zip(_) => write_zip(files, &partial, total_bytes, cancel, report),
    };
    match written.and_then(|()| fs::rename(&partial, &target).map_err(Error::from)) {
        Ok(()) => Ok(target),
        Err(error) => {
            let _ = remove_partial(&partial);
            Err(error)
        }
    }
}

/// Copies each file into the new folder `partial`, at its path.
fn write_folder(files: &[Planned], partial: &Path, total_bytes: u64, cancel: &AtomicBool, report: &mut dyn FnMut(Progress)) -> Result<()> {
    fs::create_dir(partial)?;
    let mut control = Control::new(cancel, files.len(), total_bytes, report);
    for file in files {
        control.check()?;
        let dest = partial.join(&file.path);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&file.source, &dest)?;
        control.advance(file.size)?;
    }
    Ok(())
}

/// Writes each file into the new zip `partial`, at its path. The archive holds the files only, with no manifest.
fn write_zip(files: &[Planned], partial: &Path, total_bytes: u64, cancel: &AtomicBool, report: &mut dyn FnMut(Progress)) -> Result<()> {
    let mut zip = zip::ZipWriter::new(BufWriter::new(File::create(partial)?));
    let mut control = Control::new(cancel, files.len(), total_bytes, report);
    for file in files {
        control.check()?;
        zip.start_file(&file.path, file_options(&file.path, &file.source))?;
        io::copy(&mut File::open(&file.source)?, &mut zip)?;
        control.advance(file.size)?;
    }
    zip.finish()?.flush()?;
    Ok(())
}

/// The file or folder a pack is written to before it is renamed: the target with `.partial` appended.
fn partial_path(target: &Path) -> PathBuf {
    let mut partial = target.as_os_str().to_owned();
    partial.push(".partial");
    PathBuf::from(partial)
}

/// Removes the partial folder or file at `partial`, if one is there.
fn remove_partial(partial: &Path) -> io::Result<()> {
    if partial.is_dir() {
        return fs::remove_dir_all(partial);
    }
    match fs::remove_file(partial) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

#[cfg(test)]
#[path = "../../tests/server_pack/write.rs"]
mod tests;
