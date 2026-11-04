//! Showing a file in the system's file browser with the file selected, and opening a file in its own app. Explorer,
//! Finder and the FreeDesktop file manager each have a way to select an item. Where none of them answers, the folder
//! that holds the file opens instead.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use iced::Task;

use super::LodestoneAppMessage;

/// Opens the file manager with `path` selected, on the blocking pool. A failure returns as `FolderOpened`, which the
/// root reports.
pub fn reveal_file(path: PathBuf) -> Task<LodestoneAppMessage> {
    Task::perform(
        async move {
            match tokio::task::spawn_blocking(move || select(&path)).await {
                Ok(opened) => opened.map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            }
        },
        LodestoneAppMessage::FolderOpened,
    )
}

/// Opens `path` in the app its type belongs to, on the blocking pool. A failure returns as `FolderOpened`.
pub fn open_file(path: PathBuf) -> Task<LodestoneAppMessage> {
    Task::perform(
        async move {
            match tokio::task::spawn_blocking(move || open::that_detached(&path)).await {
                Ok(opened) => opened.map_err(|error| error.to_string()),
                Err(error) => Err(error.to_string()),
            }
        },
        LodestoneAppMessage::FolderOpened,
    )
}

/// Selects `path` in Explorer. `explorer.exe` takes its argument as written, so the path is quoted here rather than by
/// the command line's own rules. It can report a failure even when it selected the file, so its status is not read.
#[cfg(windows)]
fn select(path: &Path) -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    Command::new("explorer.exe")
        .raw_arg(format!("/select,\"{}\"", path.display()))
        .spawn()
        .map(drop)
}

/// Selects `path` in Finder, with `open -R`.
#[cfg(target_os = "macos")]
fn select(path: &Path) -> io::Result<()> {
    Command::new("open").arg("-R").arg(path).status().and_then(exit_status)
}

/// Selects `path` through the FreeDesktop file manager interface, which the common file managers serve. Where no
/// file manager serves it, the folder holding the file opens.
#[cfg(all(unix, not(target_os = "macos")))]
fn select(path: &Path) -> io::Result<()> {
    let uri = format!("['{}']", file_uri(path));
    let shown = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.FileManager1",
            "--object-path",
            "/org/freedesktop/FileManager1",
            "--method",
            "org.freedesktop.FileManager1.ShowItems",
            uri.as_str(),
            "",
        ])
        .status()
        .is_ok_and(|status| status.success());
    if shown {
        return Ok(());
    }
    open::that_detached(path.parent().unwrap_or(path))
}

/// The status of a command that ran, as an error when it failed.
#[cfg(target_os = "macos")]
fn exit_status(status: std::process::ExitStatus) -> io::Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("the file browser exited with {status}")))
    }
}

/// The `file://` address of `path`, with every byte outside the URI's unreserved set escaped, so spaces and
/// non-ASCII names pass through the bus unchanged.
#[cfg(all(unix, not(target_os = "macos")))]
fn file_uri(path: &Path) -> String {
    let mut uri = String::from("file://");
    for byte in path.to_string_lossy().bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => uri.push(char::from(byte)),
            _ => uri.push_str(&format!("%{byte:02X}")),
        }
    }
    uri
}

#[cfg(test)]
#[path = "../../tests/app/reveal.rs"]
mod tests;
