//! Putting an update in place on Linux and macOS. The new executable or app bundle is copied beside the one it replaces, so
//! the last step is a single rename. A restart waits until this launcher has ended, then starts the new one.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::{Error, io_error};

/// The executable that the Linux archive and the macOS app both hold.
const EXECUTABLE: &str = "lodestone_launcher";
/// The app bundle that the macOS zip holds, named as the release workflow names it.
const BUNDLE: &str = "Lodestone.app";
/// The macOS tool that copies and unpacks bundles with their signatures, links and extended attributes intact.
const DITTO: &str = "/usr/bin/ditto";

/// Whether a file can be created in `dir`. An update can be placed only in a folder that takes new files.
pub fn can_write_to(dir: &Path) -> bool {
    let probe = dir.join(format!(".lodestone-write-check-{}", std::process::id()));
    match fs::OpenOptions::new().write(true).create_new(true).open(&probe) {
        Ok(_) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Unpacks the launcher's executable from the Linux archive into `dir`, and returns its path. The archive holds the
/// executable alone, as the release workflow builds it.
pub fn extract_executable(archive: &Path, dir: &Path) -> Result<PathBuf, Error> {
    fs::create_dir_all(dir).map_err(|source| io_error(dir, source))?;
    run(Command::new("tar").arg("-xzf").arg(archive).arg("-C").arg(dir).arg(EXECUTABLE), "tar")?;
    let executable = dir.join(EXECUTABLE);
    if !is_linux_program(&executable) {
        return Err(Error::Unpack(format!("the archive's {EXECUTABLE} is not a Linux program")));
    }
    make_executable(&executable)?;
    Ok(executable)
}

/// Unpacks `Lodestone.app` from the macOS zip into `dir`, and returns the bundle's path.
pub fn extract_bundle(archive: &Path, dir: &Path) -> Result<PathBuf, Error> {
    fs::create_dir_all(dir).map_err(|source| io_error(dir, source))?;
    run(Command::new(DITTO).arg("-x").arg("-k").arg(archive).arg(dir), "ditto")?;
    let bundle = dir.join(BUNDLE);
    if !bundle.join("Contents").join("MacOS").join(EXECUTABLE).is_file() {
        return Err(Error::Unpack(format!("the archive holds no {BUNDLE} with the launcher in it")));
    }
    Ok(bundle)
}

/// Replaces the executable at `target` with `staged`. The copy is made beside `target` first, so the last step is a rename,
/// which also works while the old executable runs.
pub fn replace_executable(staged: &Path, target: &Path) -> Result<(), Error> {
    let incoming = sibling(target, ".update")?;
    fs::copy(staged, &incoming).map_err(|source| io_error(&incoming, source))?;
    if let Err(error) = make_executable(&incoming) {
        let _ = fs::remove_file(&incoming);
        return Err(error);
    }
    fs::rename(&incoming, target).map_err(|source| {
        let _ = fs::remove_file(&incoming);
        io_error(target, source)
    })
}

/// Replaces the app bundle at `bundle` with `staged`. The new bundle is copied beside the old one, the old one is moved
/// aside, and the new one is moved into its place. If that last move fails, the old bundle goes back.
pub fn replace_bundle(staged: &Path, bundle: &Path) -> Result<(), Error> {
    let incoming = sibling(bundle, ".update")?;
    let previous = sibling(bundle, ".previous")?;
    let _ = fs::remove_dir_all(&incoming);
    let _ = fs::remove_dir_all(&previous);
    run(Command::new(DITTO).arg(staged).arg(&incoming), "ditto")?;
    if let Err(source) = fs::rename(bundle, &previous) {
        let _ = fs::remove_dir_all(&incoming);
        return Err(io_error(bundle, source));
    }
    if let Err(source) = fs::rename(&incoming, bundle) {
        let _ = fs::rename(&previous, bundle);
        let _ = fs::remove_dir_all(&incoming);
        return Err(io_error(bundle, source));
    }
    let _ = fs::remove_dir_all(&previous);
    Ok(())
}

/// Starts `program` with `arguments` once the process `pid` has ended. The command runs in its own process group, so it
/// outlives this launcher and a closing terminal does not stop it.
#[cfg(unix)]
pub fn relaunch_after_exit(pid: u32, program: &Path, arguments: &[&Path]) -> Result<(), Error> {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;

    // The shell waits while `pid` runs, then replaces itself with the program and its arguments.
    const WAIT: &str = r#"while kill -0 "$1" 2>/dev/null; do sleep 0.2; done; shift; exec "$@""#;
    Command::new("/bin/sh")
        .arg("-c")
        .arg(WAIT)
        .arg("lodestone-relaunch")
        .arg(pid.to_string())
        .arg(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .map(drop)
        .map_err(|error| Error::Launch(format!("could not start the launcher again: {error}")))
}

/// Restarting after an update is only done on Unix, where the update is placed by a rename.
#[cfg(not(unix))]
pub fn relaunch_after_exit(_pid: u32, _program: &Path, _arguments: &[&Path]) -> Result<(), Error> {
    Err(Error::Launch("the launcher restarts after an update on Linux and macOS only".to_string()))
}

/// A hidden name beside `path`, for a copy that is renamed over `path` once it is complete.
fn sibling(path: &Path, suffix: &str) -> Result<PathBuf, Error> {
    let folder = path.parent().ok_or_else(|| Error::Unpack(format!("{} has no folder", path.display())))?;
    let name = path.file_name().ok_or_else(|| Error::Unpack(format!("{} has no name", path.display())))?;
    Ok(folder.join(format!(".{}{suffix}", name.to_string_lossy())))
}

/// Runs `command`, and reports its failure with what it wrote to its error stream.
fn run(command: &mut Command, tool: &str) -> Result<(), Error> {
    let output = command
        .output()
        .map_err(|error| Error::Unpack(format!("could not run {tool}: {error}")))?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(Error::Unpack(format!("{tool} failed: {detail}")))
}

/// Whether the file starts with the ELF signature that every Linux program does.
fn is_linux_program(path: &Path) -> bool {
    let mut magic = [0u8; 4];
    File::open(path).and_then(|mut file| file.read_exact(&mut magic)).is_ok() && magic == *b"\x7fELF"
}

/// Marks a file runnable by its owner, its group and everyone else.
#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).map_err(|source| io_error(path, source))
}

/// Files have no executable bit outside Unix, so there is nothing to mark.
#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), Error> {
    Ok(())
}

#[cfg(test)]
#[path = "../tests/apply.rs"]
mod tests;
