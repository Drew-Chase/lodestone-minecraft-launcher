//! The servers an instance can run as: the file each loader's server starts from, the install that puts it on disk, and
//! the start scripts and EULA file that go beside it. Vanilla downloads the `server.jar` Mojang publishes. Fabric
//! downloads its server launcher. Forge and NeoForge run their installers with the install-server flag, and start from
//! the argument files that run leaves under `libraries`, or, before 1.17, from the jar it writes at the top of the folder.
//! Quilt has no server install here yet: its meta service offers no server jar, and its server needs its own installer.

use std::path::{Path, PathBuf};

use tokio::sync::mpsc::Sender;

use lodestone_net::download::Target;

use crate::error::LoaderError;
use crate::http::{download_files, get_json};
use crate::install::{installer_jar, run_server_installer_task};
use crate::meta::LoaderVersion;
use crate::vanilla::fetch_error;
use crate::{Forge, Loader, NeoForge};

/// The start scripts a Forge or NeoForge installer writes beside a server, which the pack leaves out.
const INSTALLER_SCRIPTS: [&str; 2] = ["run.sh", "run.bat"];
/// The file a Fabric server launcher is saved as.
const FABRIC_LAUNCHER: &str = "fabric-server-launch.jar";
/// The file a vanilla server is saved as.
const VANILLA_JAR: &str = "server.jar";
/// The folder the installer of a Forge or NeoForge server is downloaded into, and removed from once the install is done.
const INSTALLER_DIR: &str = ".installer";
/// The flag that makes the Forge installer write a server.
const FORGE_FLAG: &str = "--installServer";
/// The flag that makes the NeoForge installer write a server.
const NEOFORGE_FLAG: &str = "--install-server";
/// Where the Fabric meta service lists its installer builds.
const FABRIC_INSTALLERS: &str = "https://meta.fabricmc.net/v2/versions/installer";
/// Where the Fabric meta service serves a server launcher for a Minecraft version, a loader and an installer build.
const FABRIC_LAUNCHER_URL: &str = "https://meta.fabricmc.net/v2/versions/loader";

/// The loader a server runs for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerKind {
    /// Vanilla Minecraft: the `server.jar` Mojang publishes for the version.
    Vanilla,
    /// Fabric: its server launcher, which starts the game with the loader version it is given.
    Fabric,
    /// Quilt: no server install is offered yet.
    Quilt,
    /// Forge: its installer, run with the install-server flag.
    Forge,
    /// NeoForge: its installer, run with the install-server flag.
    NeoForge,
}

/// How a server starts once it is installed. The paths are relative to the server folder, with `/` separators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// `java -jar <jar> nogui`.
    Jar(String),
    /// `java @user_jvm_args.txt @<argument file> nogui`.
    Args {
        /// The argument file a Unix start script passes.
        unix: String,
        /// The argument file a Windows start script passes.
        windows: String,
    },
}

/// Installs the server for `kind` on Minecraft `minecraft` into `dir`, and returns how it starts. `loader_version` names
/// the loader build a Fabric, Forge or NeoForge server is for. The Forge and NeoForge installers run with `java`, and
/// their output goes to `lines` as it arrives.
pub async fn install(
    http: &reqwest::Client,
    kind: ServerKind,
    minecraft: &str,
    loader_version: Option<&str>,
    java: &Path,
    dir: &Path,
    lines: Option<&Sender<String>>,
) -> Result<Launch, LoaderError> {
    std::fs::create_dir_all(dir).map_err(|source| LoaderError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    match kind {
        ServerKind::Vanilla => vanilla(http, minecraft, dir).await,
        ServerKind::Fabric => fabric(http, minecraft, loader_version, dir).await,
        ServerKind::Quilt => Err(LoaderError::ServerUnsupported { loader: "quilt" }),
        ServerKind::Forge => {
            let Some(version) = loader_version else {
                return Err(unavailable("forge", minecraft));
            };
            let staging = dir.join(INSTALLER_DIR);
            let version = LoaderVersion::new(version, Some(minecraft.to_string()), true);
            let jars = Loader::<Forge>::new().download_client(&version, &staging, None).await?;
            run_installer("forge", FORGE_FLAG, &staging, &jars, java, dir, lines).await
        }
        ServerKind::NeoForge => {
            let Some(version) = loader_version else {
                return Err(unavailable("neoforge", minecraft));
            };
            let staging = dir.join(INSTALLER_DIR);
            let version = LoaderVersion::new(version, Some(minecraft.to_string()), true);
            let jars = Loader::<NeoForge>::new().download_client(&version, &staging, None).await?;
            run_installer("neoforge", NEOFORGE_FLAG, &staging, &jars, java, dir, lines).await
        }
    }
}

/// A Java runtime the pack carries, which the start scripts use on the system it is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedJava {
    /// The runtime's program inside the pack, with `/` separators, such as `java/bin/java`.
    pub program: String,
    /// The system the runtime is for: `Linux` or `Darwin` as `uname -s` names a Unix system, or `Windows`.
    pub os: String,
}

/// The start scripts for a server that starts as `launch`: `(start.sh, start.bat)`. The shell script runs for Linux and
/// macOS, and the batch file for Windows. Both pass the arguments they are given on to the server. A packed `java` is used
/// only by the script for the system it is for, and every other script runs the system's Java.
pub fn start_scripts(launch: &Launch, java: Option<&PackedJava>) -> (String, String) {
    let (unix, windows) = match launch {
        Launch::Jar(jar) => (format!("-jar {jar} nogui"), format!("-jar {jar} nogui")),
        Launch::Args { unix, windows } => (
            format!("@user_jvm_args.txt @{unix} nogui"),
            format!("@user_jvm_args.txt @{windows} nogui"),
        ),
    };
    let packed_shell = match java {
        Some(java) if java.os != "Windows" => format!(
            "if [ \"$(uname -s)\" = \"{}\" ] && [ -x \"{}\" ]; then JAVA=\"{}\"; fi\n",
            java.os, java.program, java.program
        ),
        _ => String::new(),
    };
    let packed_batch = match java {
        Some(java) if java.os == "Windows" => {
            let program = java.program.replace('/', "\\");
            format!("if exist \"{program}\" set \"JAVA={program}\"\r\n")
        }
        _ => String::new(),
    };
    let shell = format!("#!/bin/sh\ncd \"$(dirname \"$0\")\" || exit 1\nJAVA=java\n{packed_shell}exec \"$JAVA\" {unix} \"$@\"\n");
    let batch = format!("@echo off\r\ncd /d \"%~dp0\"\r\nset \"JAVA=java\"\r\n{packed_batch}\"%JAVA%\" {windows} %*\r\n");
    (shell, batch)
}

/// The `eula.txt` a server reads: `eula=true` only when the player accepted the Minecraft EULA.
pub fn eula(accepted: bool) -> String {
    format!("#By changing the setting below to TRUE you are indicating your agreement to our EULA (https://aka.ms/MinecraftEULA).\neula={accepted}\n")
}

/// The error for a loader that has no server for `minecraft`.
fn unavailable(loader: &'static str, minecraft: &str) -> LoaderError {
    LoaderError::ServerUnavailable {
        loader,
        minecraft: minecraft.to_string(),
    }
}

/// Downloads the vanilla `server.jar` the version manifest names for `minecraft`, checked against its SHA-1 and size.
async fn vanilla(http: &reqwest::Client, minecraft: &str, dir: &Path) -> Result<Launch, LoaderError> {
    let manifest = crate::vanilla::version_list(http).await?;
    let entry = manifest
        .versions
        .iter()
        .find(|entry| entry.id == minecraft)
        .ok_or_else(|| LoaderError::VersionNotFound {
            version: minecraft.to_string(),
            minecraft: minecraft.to_string(),
        })?;
    let version = crate::mojang::version_json(http, &entry.url)
        .await
        .map_err(|source| fetch_error(&entry.url, source))?;
    let server = version.downloads.server.ok_or_else(|| unavailable("vanilla", minecraft))?;
    let target = Target {
        url: server.url.clone(),
        path: dir.join(VANILLA_JAR),
        sha1: Some(server.sha1.clone()),
        size: Some(server.size),
    };
    download_files(http, vec![target], None).await?;
    Ok(Launch::Jar(VANILLA_JAR.to_string()))
}

/// Downloads the Fabric server launcher for `minecraft`, `loader_version` and the newest stable installer build.
async fn fabric(http: &reqwest::Client, minecraft: &str, loader_version: Option<&str>, dir: &Path) -> Result<Launch, LoaderError> {
    let Some(loader_version) = loader_version else {
        return Err(unavailable("fabric", minecraft));
    };
    let installers = get_json(http, FABRIC_INSTALLERS).await?;
    let installer = installers
        .as_array()
        .and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry.get("stable").and_then(|stable| stable.as_bool()) == Some(true))
        })
        .and_then(|entry| entry.get("version"))
        .and_then(|version| version.as_str())
        .ok_or_else(|| unavailable("fabric", minecraft))?;
    let url = format!("{FABRIC_LAUNCHER_URL}/{minecraft}/{loader_version}/{installer}/server/jar");
    let target = Target::unchecked(url, dir.join(FABRIC_LAUNCHER));
    download_files(http, vec![target], None).await?;
    Ok(Launch::Jar(FABRIC_LAUNCHER.to_string()))
}

/// Runs the installer among `jars` into `dir`, then finds the launch files the run wrote, and removes the installer
/// folder `staging`.
#[allow(clippy::too_many_arguments)]
async fn run_installer(
    loader: &'static str,
    flag: &'static str,
    staging: &Path,
    jars: &[PathBuf],
    java: &Path,
    dir: &Path,
    lines: Option<&Sender<String>>,
) -> Result<Launch, LoaderError> {
    let Some(installer) = installer_jar(jars) else {
        return Err(LoaderError::InstallerMissing { loader });
    };
    run_server_installer_task(loader, java, installer, flag, dir, lines).await?;
    let launch = launch_of(dir, loader)?;
    remove_installer_scripts(dir);
    let _ = std::fs::remove_dir_all(staging);
    Ok(launch)
}

/// Removes the start scripts the Forge and NeoForge installers write at the top of the server folder, so that only the
/// Lodestone start scripts are left. A script that is not there is skipped.
fn remove_installer_scripts(dir: &Path) {
    for name in INSTALLER_SCRIPTS {
        let _ = std::fs::remove_file(dir.join(name));
    }
}

/// The launch files an installed Forge or NeoForge server leaves: the argument files under `libraries` when both exist,
/// else the Forge jar at the top of the folder, which Forge wrote before 1.17.
fn launch_of(dir: &Path, loader: &'static str) -> Result<Launch, LoaderError> {
    let libraries = dir.join("libraries");
    if let (Some(unix), Some(windows)) = (find_named(&libraries, "unix_args.txt", dir), find_named(&libraries, "win_args.txt", dir)) {
        return Ok(Launch::Args { unix, windows });
    }
    if let Some(jar) = top_jar(dir, loader) {
        return Ok(Launch::Jar(jar));
    }
    Err(LoaderError::ServerLaunchMissing {
        loader,
        directory: dir.to_path_buf(),
    })
}

/// The first file named `name` under `folder`, as its path relative to `root` with `/` separators.
fn find_named(folder: &Path, name: &str, root: &Path) -> Option<String> {
    let entries = std::fs::read_dir(folder).ok()?;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_named(&path, name, root) {
                return Some(found);
            }
        } else if entry.file_name() == name {
            let relative = path.strip_prefix(root).ok()?;
            return Some(
                relative
                    .components()
                    .map(|part| part.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/"),
            );
        }
    }
    None
}

/// The Forge server jar at the top of `dir`, which the installer writes before 1.17: a `.jar` named after the loader
/// that is not the installer itself.
fn top_jar(dir: &Path, loader: &str) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .find(|name| name.starts_with(&format!("{loader}-")) && name.ends_with(".jar") && !name.contains("installer"))
}

#[cfg(test)]
#[path = "../tests/server.rs"]
mod tests;
