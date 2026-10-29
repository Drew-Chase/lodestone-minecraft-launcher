//! Pure assembly of the final `java` command line from a validated [`Launcher`].
//!
//! This module is a pure function from [`Launcher`] to [`std::process::Command`]:
//! all assembly logic (argument ordering, classpath joining, placeholder
//! expansion) lives here and is unit-testable with no process ever spawned.
//! Platform-conditional behavior does **not** live here — it is confined to
//! [`crate::process`].

use std::env::join_paths;
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::{Command, Stdio};

use tracing::warn;

use crate::config::Launcher;
use crate::error::LaunchError;

/// Classpath separator on Windows.
#[cfg(windows)]
const CLASSPATH_SEPARATOR: char = ';';
/// Classpath separator everywhere else.
#[cfg(not(windows))]
const CLASSPATH_SEPARATOR: char = ':';

/// Builds the command that runs the game described by `launcher`.
///
/// Argument order (DESIGN.md §5): program, `jvm_args`, the natives `-D`,
/// `-cp <joined classpath>`, `main_class`, expanded `game_args`. Both output
/// streams are piped: the drain threads in [`crate::process`] require it — an
/// undrained pipe fills and the JVM blocks on write.
pub(crate) fn build_command(launcher: &Launcher) -> Result<Command, LaunchError> {
    // The entity that joins the classpath expands `${classpath}`, so the joined
    // string and the placeholder can never disagree about the separator.
    let classpath = join_paths(launcher.classpath()).map_err(|source| LaunchError::InvalidClasspath { source })?;

    let mut command = Command::new(launcher.java_path());

    for arg in launcher.jvm_args() {
        command.arg(arg);
    }

    if let Some(natives) = launcher.natives_directory() {
        command.arg(format!("-Djava.library.path={}", natives.display()));
    }

    command.arg("-cp").arg(&classpath);
    command.arg(launcher.main_class());

    for arg in expand_placeholders(launcher, &classpath) {
        command.arg(arg);
    }

    command.current_dir(launcher.working_directory());
    command.envs(launcher.env());

    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    Ok(command)
}

/// Expands environmental placeholders in `launcher`'s game args.
///
/// Scope is environmental only (DESIGN.md §5); authentication placeholders are
/// gui's job. Expansion operates on whole tokens — an argument that is exactly
/// `${name}` — because substring templating of game args is a loader concern.
/// Unknown names, and known names whose value is unset, are warned about and
/// passed through untouched: loaders may add custom placeholders, and strict
/// erroring would make the crate refuse legitimate future input.
fn expand_placeholders(launcher: &Launcher, classpath: &OsStr) -> Vec<OsString> {
    launcher
        .game_args()
        .iter()
        .map(|arg| {
            let Some(name) = arg.strip_prefix("${").and_then(|rest| rest.strip_suffix('}')) else {
                return OsString::from(arg);
            };
            match placeholder_value(launcher, name, classpath) {
                Some(value) => value,
                None => {
                    warn!(placeholder = %arg, "unknown or unset launch placeholder was passed through untouched");
                    OsString::from(arg)
                }
            }
        })
        .collect()
}

/// Resolves one placeholder name to its value, or `None` when unknown or unset.
fn placeholder_value(launcher: &Launcher, name: &str, classpath: &OsStr) -> Option<OsString> {
    match name {
        "classpath" => Some(classpath.to_os_string()),
        "classpath_separator" => Some(OsString::from(CLASSPATH_SEPARATOR.to_string())),
        "launcher_name" => Some(OsString::from(launcher.launcher_name())),
        "launcher_version" => Some(OsString::from(launcher.launcher_version())),
        "library_directory" => non_empty(launcher.library_directory()),
        "natives_directory" => launcher.natives_directory().and_then(non_empty),
        "assets_root" => launcher.assets_directory().and_then(non_empty),
        _ => None,
    }
}

/// Treats an unset directory (empty path) like an unset placeholder value.
fn non_empty(path: &Path) -> Option<OsString> {
    if path.as_os_str().is_empty() {
        None
    } else {
        Some(path.as_os_str().to_os_string())
    }
}

#[cfg(test)]
#[path = "../tests/command.rs"]
mod tests;
