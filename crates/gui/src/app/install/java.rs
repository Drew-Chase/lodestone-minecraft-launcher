//! Bundled Java runtimes. Each Mojang runtime component installs once into
//! `meta/java/<short name>` and every instance that needs it shares that copy.

use std::path::{Path, PathBuf};

use piston_mc::java::{JavaManifest, JavaRuntime, Runtimes};
use simple_download_utility::MultiDownloadProgress;
use tokio::sync::mpsc;

use super::COMPLETE;
use super::download::{PARALLEL, Part, PartReport, relay};
use crate::app::lang;
use crate::app::paths::LauncherPaths;

/// The folder name of a Mojang runtime component: `java-runtime-delta` is `delta`, and
/// the legacy Java 8 runtime is `legacy`.
pub(super) fn short_name(component: &str) -> &str {
    match component {
        "jre-legacy" => "legacy",
        other => other.strip_prefix("java-runtime-").unwrap_or(other),
    }
}

/// The Java program inside an installed runtime: `bin/java` on Linux, `bin/java.exe` on
/// Windows, and the bundle's `Contents/Home/bin/java` on macOS.
pub(crate) fn binary(home: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        home.join("bin").join("java.exe")
    } else if cfg!(target_os = "macos") {
        home.join("jre.bundle").join("Contents").join("Home").join("bin").join("java")
    } else {
        home.join("bin").join("java")
    }
}

/// Makes sure the runtime for `component` is installed and returns its Java program and
/// its short name. An install that finished earlier is reused. One that did not is run
/// again.
pub(super) async fn ensure(paths: &LauncherPaths, component: &str, reports: &mpsc::Sender<PartReport>) -> Result<(PathBuf, String), String> {
    let name = short_name(component).to_string();
    let home = paths.java(&name);
    let _runtime = super::ensure::lock(format!("java:{name}")).await;
    if home.join(COMPLETE).is_file() {
        return Ok((binary(&home), name));
    }

    let manifest = JavaManifest::fetch()
        .await
        .map_err(|error| lang::t_with("install.java.fetch_list", &[("error", error.to_string().as_str())]))?;
    let runtime = platform_runtimes(&manifest)
        .and_then(|runtimes| runtime(runtimes, component))
        .ok_or_else(|| lang::t_with("install.java.not_published", &[("component", component)]))?;
    std::fs::create_dir_all(&home).map_err(|error| super::could_not_create(&home, error))?;

    let (sender, files) = mpsc::channel::<MultiDownloadProgress>(64);
    let work = async { runtime.install(&home, PARALLEL, Some(sender)).await.map_err(|error| error.to_string()) };
    relay(reports, Part::Java, files, work).await?;

    apply_manifest_flags(runtime, &home).await?;
    std::fs::write(home.join(COMPLETE), b"").map_err(|error| super::path_failed("install.could_not_finish", &home, error))?;
    Ok((binary(&home), name))
}

/// The runtimes this platform's manifest lists.
fn platform_runtimes(manifest: &JavaManifest) -> Option<&Runtimes> {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some(&manifest.windows_x64)
    } else if cfg!(all(target_os = "windows", target_arch = "x86")) {
        Some(&manifest.windows_x86)
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        Some(&manifest.windows_arm64)
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some(&manifest.linux)
    } else if cfg!(all(target_os = "linux", target_arch = "x86")) {
        Some(&manifest.linux_i386)
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some(&manifest.macos)
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some(&manifest.macos_arm64)
    } else {
        None
    }
}

/// Every runtime component, as Mojang names them.
const COMPONENTS: [&str; 7] = [
    "java-runtime-alpha",
    "java-runtime-beta",
    "java-runtime-gamma",
    "java-runtime-delta",
    "java-runtime-gamma-snapshot",
    "java-runtime-epsilon",
    "jre-legacy",
];

/// The components this platform publishes, each with its Java major version when its
/// version name gives one, newest Java first.
pub(super) async fn platform_components() -> Result<Vec<(String, Option<u8>)>, String> {
    let manifest = JavaManifest::fetch()
        .await
        .map_err(|error| lang::t_with("install.java.fetch_list", &[("error", error.to_string().as_str())]))?;
    let Some(runtimes) = platform_runtimes(&manifest) else {
        return Ok(Vec::new());
    };
    let mut components: Vec<(String, Option<u8>)> = COMPONENTS
        .into_iter()
        .filter_map(|component| runtime(runtimes, component).map(|runtime| (component.to_string(), major(&runtime.version.name))))
        .collect();
    components.sort_by_key(|(_, major)| std::cmp::Reverse(*major));
    Ok(components)
}

/// The leading number of a runtime's version name: `21.0.5` gives 21.
fn major(name: &str) -> Option<u8> {
    name.split(['.', '+', '-', ' ']).next()?.parse().ok()
}

/// The first runtime a component names in a platform's manifest.
fn runtime<'a>(runtimes: &'a Runtimes, component: &str) -> Option<&'a JavaRuntime> {
    let list = match component {
        "java-runtime-alpha" => &runtimes.alpha,
        "java-runtime-beta" => &runtimes.beta,
        "java-runtime-gamma" => &runtimes.gamma,
        "java-runtime-delta" => &runtimes.delta,
        "java-runtime-gamma-snapshot" => &runtimes.gamma_snapshot,
        "java-runtime-epsilon" => &runtimes.epsilon,
        "jre-legacy" => &runtimes.legacy,
        _ => return None,
    };
    list.first()
}

/// One entry of a runtime's file list, with the flags the downloader ignores.
struct Entry {
    name: String,
    kind: String,
    executable: bool,
    target: Option<String>,
}

/// The downloader writes files and nothing else. Mojang's file list also marks the
/// executables and the links, which it leaves out, so they are applied here.
async fn apply_manifest_flags(runtime: &JavaRuntime, home: &Path) -> Result<(), String> {
    let url = &runtime.manifest.url;
    let text = reqwest::get(url)
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| lang::t_with("install.java.fetch_file", &[("url", url.as_str()), ("error", error.to_string().as_str())]))?
        .text()
        .await
        .map_err(|error| lang::t_with("install.java.read_file", &[("url", url.as_str()), ("error", error.to_string().as_str())]))?;
    let entries = parse_entries(&text)?;
    let home = home.to_path_buf();
    tokio::task::spawn_blocking(move || apply(&home, &entries))
        .await
        .map_err(|error| error.to_string())?
}

/// Reads the `files` map of a runtime's file list.
fn parse_entries(text: &str) -> Result<Vec<Entry>, String> {
    let json: serde_json::Value =
        serde_json::from_str(text).map_err(|error| lang::t_with("install.java.list_parse", &[("error", error.to_string().as_str())]))?;
    let files = json
        .get("files")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| lang::t("install.java.list_empty").to_owned())?;
    Ok(files
        .iter()
        .map(|(name, entry)| Entry {
            name: name.clone(),
            kind: entry.get("type").and_then(serde_json::Value::as_str).unwrap_or("file").to_string(),
            executable: entry.get("executable").and_then(serde_json::Value::as_bool).unwrap_or(false),
            target: entry.get("target").and_then(serde_json::Value::as_str).map(str::to_string),
        })
        .collect())
}

/// Creates the directories, links and executables of a runtime that is already downloaded.
fn apply(home: &Path, entries: &[Entry]) -> Result<(), String> {
    for entry in entries {
        let path = home.join(&entry.name);
        match (entry.kind.as_str(), &entry.target) {
            ("directory", _) => {
                std::fs::create_dir_all(&path).map_err(|error| super::could_not_create(&path, error))?;
            }
            ("link", Some(target)) => link(&path, target)?,
            _ if entry.executable => make_executable(&path)?,
            _ => {}
        }
    }
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .map_err(|error| super::path_failed("install.could_not_make_executable", path, error))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn link(path: &Path, target: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| super::could_not_create(parent, error))?;
    }
    if path.symlink_metadata().is_ok() {
        std::fs::remove_file(path).map_err(|error| super::path_failed("install.could_not_replace", path, error))?;
    }
    std::os::unix::fs::symlink(target, path).map_err(|error| super::path_failed("install.could_not_link", path, error))
}

#[cfg(not(unix))]
fn link(_path: &Path, _target: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/app/install/java.rs"]
mod tests;
