//! Bundled Java runtimes. Each Mojang runtime component installs once into
//! `meta/java/<short name>` and every instance that needs it shares that copy.
//!
//! A runtime's file list comes from Mojang's runtime manifest, which must hash to the SHA-1 the platform's list gives it.
//! Each file the list names is fetched through the launcher's checked downloader, and the directories, links and executable
//! bits are applied once the files are in place.

use std::path::{Path, PathBuf};

use piston_mc::java::{JavaManifest, JavaRuntime, Runtimes};
use serde_json::Value;
use tokio::sync::mpsc;

use super::COMPLETE;
use super::download::{self, Missing, Part, PartReport};
use crate::app::lang;
use crate::app::launch::libraries::Item;
use crate::app::paths::LauncherPaths;

/// Mojang's list of the Java runtimes each platform publishes.
const LIST_URL: &str = "https://piston-meta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

/// Windows: a console program started from a window opens a console unless it is created without one.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

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

/// Where the runtime for `component` is installed, when that install finished. The Settings page lists these.
pub(crate) fn installed_home(paths: &LauncherPaths, component: &str) -> Option<PathBuf> {
    let home = paths.java(short_name(component));
    home.join(COMPLETE).is_file().then_some(home)
}

/// Makes sure the runtime for `component` is installed and returns its Java program and
/// its short name. An install that finished earlier is reused. One that did not is run
/// again, and the files it already has are checked and kept.
pub(super) async fn ensure(paths: &LauncherPaths, component: &str, reports: &mpsc::Sender<PartReport>) -> Result<(PathBuf, String), String> {
    let name = short_name(component).to_string();
    let home = paths.java(&name);
    let _runtime = super::ensure::lock(format!("java:{name}")).await;
    if home.join(COMPLETE).is_file() {
        return Ok((binary(&home), name));
    }

    let client = lodestone_net::client();
    let manifest = fetch_list(&client).await?;
    let runtime = platform_runtimes(&manifest)
        .and_then(|runtimes| runtime(runtimes, component))
        .ok_or_else(|| lang::t_with("install.java.not_published", &[("component", component)]))?;
    std::fs::create_dir_all(&home).map_err(|error| super::could_not_create(&home, error))?;
    install_runtime(&client, runtime, &home, reports).await?;
    lodestone_fs::write_atomic(&home.join(COMPLETE), b"").map_err(|error| super::path_failed("install.could_not_finish", &home, error))?;
    Ok((binary(&home), name))
}

/// Installs the files of `runtime` into `home`: the files its list names, then the directories, links and executable bits.
pub(super) async fn install_runtime(
    client: &reqwest::Client,
    runtime: &JavaRuntime,
    home: &Path,
    reports: &mpsc::Sender<PartReport>,
) -> Result<(), String> {
    let entries = fetch_entries(client, runtime).await?;
    let items = file_items(home, &entries)?;
    let total = items.len();
    let Missing { missing, present } = download::missing(items).await?;
    download::fetch(client, missing, Part::Java, present, total, reports).await?;

    let home = home.to_path_buf();
    tokio::task::spawn_blocking(move || apply(&home, &entries))
        .await
        .map_err(|error| error.to_string())?
}

/// Fetches Mojang's list of runtimes.
async fn fetch_list(client: &reqwest::Client) -> Result<JavaManifest, String> {
    lodestone_net::get_json(client, LIST_URL)
        .await
        .map_err(|error| lang::t_with("install.java.fetch_list", &[("error", error.to_string().as_str())]))
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
    let manifest = fetch_list(&lodestone_net::client()).await?;
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

/// One entry of a runtime's file list: a file, a directory or a link.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    name: String,
    kind: String,
    executable: bool,
    target: Option<String>,
    /// Where a file's bytes come from. A file the list gives no download for is left out.
    raw: Option<Raw>,
}

/// A file's raw download, with the SHA-1 and size it must have.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Raw {
    url: String,
    sha1: String,
    size: u64,
}

/// Reads the file list of `runtime`. The list must hash to the SHA-1 its manifest names, so a list that was changed on the
/// way is not trusted.
async fn fetch_entries(client: &reqwest::Client, runtime: &JavaRuntime) -> Result<Vec<Entry>, String> {
    let url = &runtime.manifest.url;
    let bytes = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| lang::t_with("install.java.fetch_file", &[("url", url.as_str()), ("error", error.to_string().as_str())]))?
        .bytes()
        .await
        .map_err(|error| lang::t_with("install.java.read_file", &[("url", url.as_str()), ("error", error.to_string().as_str())]))?;
    if !lodestone_net::download::sha1_hex(&bytes).eq_ignore_ascii_case(&runtime.manifest.sha1) {
        return Err(lang::t_with("install.java.list_checksum", &[("url", url.as_str())]));
    }
    parse_entries(&String::from_utf8_lossy(&bytes))
}

/// Reads the `files` map of a runtime's file list.
fn parse_entries(text: &str) -> Result<Vec<Entry>, String> {
    let json: Value =
        serde_json::from_str(text).map_err(|error| lang::t_with("install.java.list_parse", &[("error", error.to_string().as_str())]))?;
    let files = json
        .get("files")
        .and_then(Value::as_object)
        .ok_or_else(|| lang::t("install.java.list_empty").to_owned())?;
    // The names are Mojang's, but each is still a path under the runtime's folder, so one that leaves the folder is refused.
    files
        .iter()
        .map(|(name, entry)| {
            if !lodestone_platforms::paths::is_contained(name) {
                return Err(lang::t_with("install.unsafe_path", &[("path", name.as_str())]));
            }
            Ok(Entry {
                name: name.clone(),
                kind: entry.get("type").and_then(Value::as_str).unwrap_or("file").to_string(),
                executable: entry.get("executable").and_then(Value::as_bool).unwrap_or(false),
                target: entry.get("target").and_then(Value::as_str).map(str::to_string),
                raw: raw_of(entry),
            })
        })
        .collect()
}

/// The raw download of one list entry, when it names one completely.
fn raw_of(entry: &Value) -> Option<Raw> {
    let raw = entry.get("downloads")?.get("raw")?;
    Some(Raw {
        url: raw.get("url")?.as_str()?.to_string(),
        sha1: raw.get("sha1")?.as_str()?.to_string(),
        size: raw.get("size")?.as_u64()?,
    })
}

/// The downloads for the files of `entries`, each placed under `home`.
fn file_items(home: &Path, entries: &[Entry]) -> Result<Vec<Item>, String> {
    entries
        .iter()
        .filter(|entry| entry.kind == "file")
        .map(|entry| {
            let raw = entry
                .raw
                .as_ref()
                .ok_or_else(|| lang::t_with("install.no_download", &[("path", entry.name.as_str())]))?;
            Ok(Item {
                url: Some(raw.url.clone()),
                path: home.join(&entry.name),
                sha1: Some(raw.sha1.clone()),
                size: Some(raw.size),
            })
        })
        .collect()
}

/// Creates the directories, links and executables of a runtime whose files are downloaded.
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

/// A Java program already on this machine, and the major version it reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SystemJava {
    /// The program, `java` or `java.exe`.
    pub(crate) program: PathBuf,
    /// The major version it reports, such as 17.
    pub(crate) major: u32,
}

/// The first Java on `JAVA_HOME` or `PATH` that runs and is at least `at_least` major. `None` when the machine has none.
pub(crate) async fn system(at_least: u32) -> Option<SystemJava> {
    tokio::task::spawn_blocking(move || find_system(at_least)).await.ok().flatten()
}

fn find_system(at_least: u32) -> Option<SystemJava> {
    let exe = if cfg!(target_os = "windows") { "java.exe" } else { "java" };
    let home = std::env::var_os("JAVA_HOME").map(|home| PathBuf::from(home).join("bin").join(exe));
    let path = std::env::var_os("PATH").map(|paths| std::env::split_paths(&paths).map(|dir| dir.join(exe)).collect::<Vec<_>>());
    home.into_iter()
        .chain(path.into_iter().flatten())
        .filter(|program| program.is_file())
        .find_map(|program| {
            let major = major_version(&program).filter(|major| *major >= at_least)?;
            Some(SystemJava { program, major })
        })
}

/// The major version a Java program reports with `-version`, which writes to stderr.
fn major_version(program: &Path) -> Option<u32> {
    let mut command = std::process::Command::new(program);
    command.arg("-version");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command.output().ok()?;
    parse_major(&String::from_utf8_lossy(&output.stderr))
}

/// The major version in `-version` output. The version line can come after lines a JVM prints for itself, such as
/// "Picked up JAVA_TOOL_OPTIONS", so the first line that names a version is the one read.
fn parse_major(output: &str) -> Option<u32> {
    let line = output.lines().find(|line| line.contains("version \""))?;
    let version = line.split('"').nth(1)?;
    let mut parts = version.split(|c: char| !c.is_ascii_digit());
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 { parts.next()?.parse().ok() } else { Some(first) }
}

#[cfg(test)]
#[path = "../../../tests/app/install/java.rs"]
mod tests;
