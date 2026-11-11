//! Mojang's Java runtimes, and the system Java the launcher retries an installer with. These are ports of
//! `crates/gui/src/app/install/java.rs` and `install/loader.rs::run`, which this leaf crate cannot call.

use std::path::{Component, Path, PathBuf};

use futures_util::{StreamExt, TryStreamExt};
use lodestone_net::download::{self, Target};
use piston_mc::java::{JavaManifest, JavaRuntime, Runtimes};
use piston_mc::version_manifest::JavaVersion;
use serde_json::Value;

/// Mojang's list of the Java runtimes each platform publishes. The launcher reads the same list.
pub(crate) const LIST_URL: &str = "https://piston-meta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

/// The marker a finished runtime install leaves. The launcher uses the same name.
const COMPLETE: &str = ".complete";

/// How many runtime files download at once.
const FILE_CONCURRENCY: usize = 16;

/// The runtime component a Minecraft version runs on, and the Java major version that component is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Requirement {
    pub(crate) component: String,
    pub(crate) major: u32,
}

impl Requirement {
    /// The requirement of a version's `javaVersion`. A version that names none runs on Mojang's Java 8, as the launcher reads it.
    pub(crate) fn from_mojang(java: Option<&JavaVersion>) -> Self {
        match java {
            Some(java) => Self {
                component: java.component.clone(),
                major: u32::from(java.major_version),
            },
            None => Self {
                component: "jre-legacy".to_string(),
                major: 8,
            },
        }
    }
}

/// Mojang's runtime list for this platform's manifest.
pub(crate) async fn fetch_list(client: &reqwest::Client) -> Result<JavaManifest, String> {
    lodestone_net::get_json(client, LIST_URL)
        .await
        .map_err(|error| format!("could not read Mojang's Java runtime list: {error}"))
}

/// Installs the runtime for `component` under `root/<component>`, unless a finished install is there already, and returns
/// the Java program inside it. The files of an install that did not finish are fetched again.
pub(crate) async fn install_runtime(client: &reqwest::Client, list: &JavaManifest, component: &str, root: &Path) -> Result<PathBuf, String> {
    let home = root.join(component);
    if home.join(COMPLETE).is_file() {
        return Ok(binary(&home));
    }
    let runtime = platform_runtimes(list)
        .and_then(|runtimes| runtime(runtimes, component))
        .ok_or_else(|| format!("Mojang publishes no {component} runtime for this platform"))?;
    std::fs::create_dir_all(&home).map_err(|error| format!("could not create {}: {error}", home.display()))?;
    let entries = fetch_entries(client, runtime).await?;
    let targets = file_targets(&home, &entries)?;
    futures_util::stream::iter(targets)
        .map(|target| async move {
            download::fetch(client, &target, |_, _| {})
                .await
                .map(drop)
                .map_err(|error| error.to_string())
        })
        .buffer_unordered(FILE_CONCURRENCY)
        .try_collect::<Vec<()>>()
        .await?;
    apply(&home, &entries)?;
    std::fs::write(home.join(COMPLETE), b"").map_err(|error| format!("could not mark {} finished: {error}", home.display()))?;
    Ok(binary(&home))
}

/// The Java program inside an installed runtime: `bin/java` on Linux, `bin/java.exe` on Windows, and the bundle's
/// `Contents/Home/bin/java` on macOS.
pub(crate) fn binary(home: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        home.join("bin").join("java.exe")
    } else if cfg!(target_os = "macos") {
        home.join("jre.bundle").join("Contents").join("Home").join("bin").join("java")
    } else {
        home.join("bin").join("java")
    }
}

/// The runtimes this platform's entry of the list holds.
fn platform_runtimes(list: &JavaManifest) -> Option<&Runtimes> {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some(&list.windows_x64)
    } else if cfg!(all(target_os = "windows", target_arch = "x86")) {
        Some(&list.windows_x86)
    } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
        Some(&list.windows_arm64)
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some(&list.linux)
    } else if cfg!(all(target_os = "linux", target_arch = "x86")) {
        Some(&list.linux_i386)
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some(&list.macos)
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some(&list.macos_arm64)
    } else {
        None
    }
}

/// The first runtime a component names in a platform's entry.
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
    /// Where a file's bytes come from. A file the list gives no download for cannot be installed.
    raw: Option<Raw>,
}

/// A file's raw download, with the SHA-1 and size it must have.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Raw {
    url: String,
    sha1: String,
    size: u64,
}

/// Reads a runtime's file list. The list must hash to the SHA-1 its manifest names, so a list changed on the way is refused.
async fn fetch_entries(client: &reqwest::Client, runtime: &JavaRuntime) -> Result<Vec<Entry>, String> {
    let url = runtime.manifest.url.as_str();
    let bytes = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| format!("could not fetch {url}: {error}"))?
        .bytes()
        .await
        .map_err(|error| format!("could not read {url}: {error}"))?;
    if !download::sha1_hex(&bytes).eq_ignore_ascii_case(&runtime.manifest.sha1) {
        return Err(format!("{url} does not match the SHA-1 Mojang's list gives it"));
    }
    parse_entries(&String::from_utf8_lossy(&bytes))
}

/// Reads the `files` map of a runtime's file list.
fn parse_entries(text: &str) -> Result<Vec<Entry>, String> {
    let json: Value = serde_json::from_str(text).map_err(|error| format!("could not parse Mojang's file list: {error}"))?;
    let files = json
        .get("files")
        .and_then(Value::as_object)
        .ok_or_else(|| "Mojang's file list names no files".to_string())?;
    files
        .iter()
        .map(|(name, entry)| {
            if !is_contained(name) {
                return Err(format!("Mojang's file list names {name}, which is outside the runtime"));
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

/// Whether `name` stays inside the runtime's folder: a relative path with no `..`, root or prefix in it.
fn is_contained(name: &str) -> bool {
    !name.is_empty() && Path::new(name).components().all(|part| matches!(part, Component::Normal(_)))
}

/// The raw download of one entry, when it names one completely.
fn raw_of(entry: &Value) -> Option<Raw> {
    let raw = entry.get("downloads")?.get("raw")?;
    Some(Raw {
        url: raw.get("url")?.as_str()?.to_string(),
        sha1: raw.get("sha1")?.as_str()?.to_string(),
        size: raw.get("size")?.as_u64()?,
    })
}

/// The downloads for the files of `entries`, each placed under `home`.
fn file_targets(home: &Path, entries: &[Entry]) -> Result<Vec<Target>, String> {
    entries
        .iter()
        .filter(|entry| entry.kind == "file")
        .map(|entry| {
            let raw = entry.raw.as_ref().ok_or_else(|| format!("Mojang gives {} no download", entry.name))?;
            Ok(Target {
                url: raw.url.clone(),
                path: home.join(&entry.name),
                sha1: Some(raw.sha1.clone()),
                size: Some(raw.size),
            })
        })
        .collect()
}

/// Creates the directories, links and executable bits of a runtime whose files are in place.
fn apply(home: &Path, entries: &[Entry]) -> Result<(), String> {
    for entry in entries {
        let path = home.join(&entry.name);
        match (entry.kind.as_str(), &entry.target) {
            ("directory", _) => std::fs::create_dir_all(&path).map_err(|error| format!("could not create {}: {error}", path.display()))?,
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
        .map_err(|error| format!("could not make {} executable: {error}", path.display()))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn link(path: &Path, target: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    if path.symlink_metadata().is_ok() {
        std::fs::remove_file(path).map_err(|error| format!("could not replace {}: {error}", path.display()))?;
    }
    std::os::unix::fs::symlink(target, path).map_err(|error| format!("could not link {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn link(_path: &Path, _target: &str) -> Result<(), String> {
    Ok(())
}

/// A Java program on this machine, and the major version it reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SystemJava {
    pub(crate) program: PathBuf,
    pub(crate) major: u32,
}

/// The first Java on `JAVA_HOME` or `PATH` that runs and is at least `at_least` major, which is how the launcher finds the
/// Java it retries an installer with.
pub(crate) async fn system_java(at_least: u32) -> Option<SystemJava> {
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
    let output = std::process::Command::new(program).arg("-version").output().ok()?;
    parse_major(&String::from_utf8_lossy(&output.stderr))
}

/// The major version in `-version` output. A JVM can print lines before its version line, so the first line that names a
/// version is the one read. A version that starts with `1.` (Java 8 and older) gives the number after it.
pub(crate) fn parse_major(output: &str) -> Option<u32> {
    let line = output.lines().find(|line| line.contains("version \""))?;
    let version = line.split('"').nth(1)?;
    let mut parts = version.split(|c: char| !c.is_ascii_digit());
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 { parts.next()?.parse().ok() } else { Some(first) }
}
