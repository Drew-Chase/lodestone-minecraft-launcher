//! The server pack's export job. When the pack includes a server, the job installs it into a staging folder first, with its
//! start scripts and EULA file, and the pack takes those files beside the mods. When the pack includes the Java runtime,
//! that runtime's files are copied in under `java/`, and the start scripts use it. Then the job plans the files and writes
//! them on the blocking pool, reporting each file. A cancel stops the write at the next file and leaves no partial output.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use lodestone_export::plan::{Planned, Selection, expand};
use lodestone_export::progress::Progress;
use lodestone_export::server_pack::plan::plan;
use lodestone_export::server_pack::write::{Output, write};
use lodestone_loaders::server::{PackedJava, ServerKind, eula, install, start_scripts};
use sipper::{Sender, Sipper, sipper};
use tokio::sync::mpsc;

use crate::app::install::{BundledJava, COMPLETE, bundled_java};
use crate::app::pages::server_pack::ExportTick;
use crate::app::paths::{self, LauncherPaths};

/// The file the Unix start script is saved as.
const START_SH: &str = "start.sh";
/// The file the Windows start script is saved as.
const START_BAT: &str = "start.bat";
/// The file the server reads for the EULA.
const EULA_TXT: &str = "eula.txt";
/// The folder the pack's Java runtime is copied into.
const JAVA_DIR: &str = "java";

/// The server the pack includes.
#[derive(Debug, Clone)]
pub struct ServerRequest {
    /// The loader the server runs for.
    pub kind: ServerKind,
    /// The Minecraft version the server runs.
    pub minecraft: String,
    /// The loader's own version, for a Fabric, Forge or NeoForge server.
    pub loader_version: Option<String>,
    /// Whether the player accepted the Minecraft EULA, which the server file says.
    pub eula: bool,
    /// Whether the pack carries the Java runtime the Minecraft version needs, and its start scripts use that runtime.
    pub include_java: bool,
    /// The launcher's folders. The Java runtimes are kept under the meta folder.
    pub paths: LauncherPaths,
}

/// What the pack takes, and where it is written.
#[derive(Debug, Clone)]
pub struct Request {
    /// The instance folder the pack is taken from.
    pub folder: PathBuf,
    /// The top-level folders the pack takes.
    pub selection: Selection,
    /// The mod jars the pack takes.
    pub jars: Vec<PathBuf>,
    /// Where the pack is written.
    pub output: Output,
    /// The server to install beside the mods, when the pack includes one.
    pub server: Option<ServerRequest>,
}

/// Writes the pack described by `request`, and returns the target it wrote and the number of files in it. The write
/// stops at the next file once `cancel` is set.
pub fn run(request: Request, cancel: Arc<AtomicBool>) -> impl Sipper<Result<(PathBuf, usize), String>, ExportTick> {
    sipper(move |mut progress: Sender<ExportTick>| async move {
        let staging = request.server.as_ref().map(|_| staging_dir());
        let mut runtime_home = None;
        if let (Some(server), Some(staging)) = (request.server.as_ref(), staging.as_ref()) {
            // The install has no total of its own, so the screen shows it as an open-ended step.
            progress.send(ExportTick::default()).await;
            match install_server(server, staging).await {
                Ok(home) => runtime_home = home,
                Err(error) => {
                    let _ = std::fs::remove_dir_all(staging);
                    return Err(error);
                }
            }
        }

        let (reports, mut received) = mpsc::unbounded_channel::<ExportTick>();
        let work_staging = staging.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let mut files = plan(&request.folder, &request.selection, &request.jars).map_err(|error| error.to_string())?;
            if let Some(staging) = &work_staging {
                files.extend(expand(staging, &Selection::default()).map_err(|error| error.to_string())?);
            }
            if let Some(home) = &runtime_home {
                files.extend(runtime_files(home).map_err(|error| error.to_string())?);
            }
            files.sort_by(|a, b| a.path.cmp(&b.path));
            let count = files.len();
            let mut report = |state: Progress| {
                let _ = reports.send(ExportTick {
                    done: state.files,
                    total: state.total_files,
                });
            };
            let target = write(&files, &request.output, &request.folder, &cancel, &mut report).map_err(|error| error.to_string())?;
            Ok((target, count))
        });
        while let Some(tick) = received.recv().await {
            progress.send(tick).await;
        }
        let result = match handle.await {
            Ok(result) => result,
            Err(error) => Err(error.to_string()),
        };
        if let Some(staging) = &staging {
            let _ = std::fs::remove_dir_all(staging);
        }
        result
    })
}

/// A fresh folder under the launcher's cache folder for a server being installed.
fn staging_dir() -> PathBuf {
    let root = paths::cache_dir().unwrap_or_else(std::env::temp_dir);
    root.join("server-pack").join(format!("{:032x}", fastrand::u128(..)))
}

/// Installs the server into `staging`, and writes its start scripts and EULA file beside it. A Forge or NeoForge installer
/// runs on the Java the Minecraft version needs, and so does a pack that includes its runtime. The result is the runtime's
/// folder when the pack includes it, so the export can copy its files in.
async fn install_server(server: &ServerRequest, staging: &Path) -> Result<Option<PathBuf>, String> {
    let client = lodestone_net::client();
    let needs_java = matches!(server.kind, ServerKind::Forge | ServerKind::NeoForge);
    let runtime = if needs_java || server.include_java {
        Some(bundled_java(&server.paths, &client, &server.minecraft).await?)
    } else {
        None
    };
    let program = runtime.as_ref().map_or_else(PathBuf::new, |runtime| runtime.program.clone());
    let launch = install(
        &client,
        server.kind,
        &server.minecraft,
        server.loader_version.as_deref(),
        &program,
        staging,
        None,
    )
    .await
    .map_err(|error| error.to_string())?;
    let packed = runtime.as_ref().filter(|_| server.include_java).and_then(packed_java);
    let (shell, batch) = start_scripts(&launch, packed.as_ref());
    let shell_path = staging.join(START_SH);
    std::fs::write(&shell_path, shell).map_err(|error| error.to_string())?;
    make_executable(&shell_path)?;
    std::fs::write(staging.join(START_BAT), batch).map_err(|error| error.to_string())?;
    std::fs::write(staging.join(EULA_TXT), eula(server.eula)).map_err(|error| error.to_string())?;
    Ok(runtime.filter(|_| server.include_java).map(|runtime| runtime.home))
}

/// The runtime's program as the pack holds it, under `java/`, and the system it is for.
fn packed_java(runtime: &BundledJava) -> Option<PackedJava> {
    let relative = runtime.program.strip_prefix(&runtime.home).ok()?;
    let program = std::iter::once(JAVA_DIR.to_string())
        .chain(relative.components().map(|part| part.as_os_str().to_string_lossy().into_owned()))
        .collect::<Vec<_>>()
        .join("/");
    Some(PackedJava {
        program,
        os: host_os().to_string(),
    })
}

/// The system this launcher runs on, named as the start scripts check it: `Windows`, `Darwin` on macOS, or `Linux`.
fn host_os() -> &'static str {
    if cfg!(windows) {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "Darwin"
    } else {
        "Linux"
    }
}

/// The files of the Java runtime at `home`, each under `java/` in the pack. A link is taken as the file it points to, so the
/// runtime still works inside the pack. A link to a folder is not followed, and the runtime's install marker is left out.
fn runtime_files(home: &Path) -> io::Result<Vec<Planned>> {
    let mut files = Vec::new();
    collect_runtime(home, home, &mut files)?;
    Ok(files)
}

/// Adds the files of `folder`, and of the folders under it, to `files`.
fn collect_runtime(home: &Path, folder: &Path, files: &mut Vec<Planned>) -> io::Result<()> {
    for entry in std::fs::read_dir(folder)? {
        let entry = entry?;
        if entry.file_name().to_string_lossy() == COMPLETE {
            continue;
        }
        let path = entry.path();
        // A dangling link has no file to copy, so it is left out.
        let Ok(metadata) = std::fs::metadata(&path) else {
            continue;
        };
        if metadata.is_dir() {
            if !entry.file_type()?.is_symlink() {
                collect_runtime(home, &path, files)?;
            }
        } else if metadata.is_file()
            && let Some(name) = pack_path(home, &path)
        {
            files.push(Planned {
                path: name,
                source: path,
                size: metadata.len(),
            });
        }
    }
    Ok(())
}

/// The path a file of the runtime at `home` takes in the pack: `java/` and the file's path under `home`, with `/` separators.
fn pack_path(home: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(home).ok()?;
    let parts: Vec<String> = relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    Some(format!("{JAVA_DIR}/{}", parts.join("/")))
}

/// Marks a start script runnable on Unix. Windows runs scripts without a mode.
#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).map_err(|error| error.to_string())
}

/// Marks a start script runnable on Unix. Windows runs scripts without a mode.
#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/app/server_pack/job.rs"]
mod tests;
