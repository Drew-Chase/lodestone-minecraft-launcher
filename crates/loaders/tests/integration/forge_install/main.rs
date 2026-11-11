//! The Forge loader matrix: every Minecraft version's latest Forge build, downloaded and installed the way the launcher installs
//! it, with one row per version in `target/tmp/forge-install/report.md`.
//!
//! The matrix needs the network and runs real installers, so it is `#[ignore]`d. Run it with `just forge-matrix`, or with
//! `cargo test -p lodestone_loaders --test forge_install -- --ignored --nocapture`. `LODESTONE_FORGE_JOBS` sets how many
//! installs run at once (default 4), and `LODESTONE_FORGE_VERSIONS`, a comma-separated list of Minecraft versions, narrows the
//! run. The other tests in this file need no network and run in CI.

mod java;
mod report;
mod verify;

#[path = "../fixture/mod.rs"]
mod fixture;

use std::collections::{BTreeMap, HashSet};
use std::fmt::Display;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use futures_util::StreamExt;
use lodestone_loaders::{Forge, InstalledVersion, Loader, LoaderError, LoaderVersion};
use piston_mc::version_manifest::{LibraryItem, LibraryItemDownloader, Rule, VersionManifest};
use report::{Outcome, Row};
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

/// The environment variable that sets how many installs run at once.
const JOBS_VAR: &str = "LODESTONE_FORGE_JOBS";
/// The environment variable that narrows the run to some Minecraft versions, as a comma-separated list.
const VERSIONS_VAR: &str = "LODESTONE_FORGE_VERSIONS";
const DEFAULT_JOBS: usize = 4;
/// How many library files the vanilla stage downloads at once, for each install.
const LIBRARY_CONCURRENCY: u16 = 16;

/// The Minecraft versions whose latest Forge build has no installer on maven.minecraftforge.net, checked on 2026-11-11. Each
/// must fail its download with a 404 on that installer. When one starts to download, the matrix fails, so its entry comes out.
const KNOWN_GAPS: &[(&str, &str)] = &[
    ("1.1", "no installer on maven.minecraftforge.net"),
    ("1.2.3", "no installer on maven.minecraftforge.net"),
    ("1.2.4", "no installer on maven.minecraftforge.net"),
    ("1.2.5", "no installer on maven.minecraftforge.net"),
    ("1.3.2", "no installer on maven.minecraftforge.net"),
    ("1.4.0", "no installer on maven.minecraftforge.net"),
    ("1.4.1", "no installer on maven.minecraftforge.net"),
    ("1.4.2", "no installer on maven.minecraftforge.net"),
    ("1.4.3", "no installer on maven.minecraftforge.net"),
    ("1.4.4", "no installer on maven.minecraftforge.net"),
    ("1.4.5", "no installer on maven.minecraftforge.net"),
    ("1.4.6", "no installer on maven.minecraftforge.net"),
    ("1.4.7", "only a universal.zip on maven.minecraftforge.net"),
    ("1.5", "only a universal.zip on maven.minecraftforge.net"),
    ("1.5.1", "only a universal.zip on maven.minecraftforge.net"),
];

/// One Minecraft version: its latest Forge build, and the reason it is a known gap when it is one.
#[derive(Debug, Clone)]
struct Case {
    minecraft: String,
    version: LoaderVersion,
    gap: Option<&'static str>,
}

/// The latest build of each Minecraft version, which is the last one the index lists for it. For every version that has a
/// `-latest` promotion, that is the promoted build. Sorted by version, and narrowed to `only` when it is given.
fn select(all: &[LoaderVersion], only: Option<&HashSet<String>>) -> Result<Vec<Case>, String> {
    let mut latest: BTreeMap<String, LoaderVersion> = BTreeMap::new();
    for version in all {
        if let Some(minecraft) = &version.minecraft {
            latest.insert(minecraft.clone(), version.clone());
        }
    }
    if let Some(only) = only {
        let mut unknown: Vec<&str> = only.iter().map(String::as_str).filter(|name| !latest.contains_key(*name)).collect();
        unknown.sort_unstable();
        if !unknown.is_empty() {
            return Err(format!(
                "{VERSIONS_VAR} names Minecraft versions that Forge does not list: {}",
                unknown.join(", ")
            ));
        }
    }
    let mut cases: Vec<Case> = latest
        .into_iter()
        .filter(|(minecraft, _)| only.is_none_or(|only| only.contains(minecraft)))
        .map(|(minecraft, version)| {
            let gap = KNOWN_GAPS.iter().find(|(name, _)| *name == minecraft).map(|(_, reason)| *reason);
            Case { minecraft, version, gap }
        })
        .collect();
    cases.sort_by_key(|case| version_key(&case.minecraft));
    Ok(cases)
}

/// A sort key that orders version strings by their numbers, so `1.9` comes before `1.10`.
fn version_key(version: &str) -> Vec<u64> {
    version
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|part| part.parse().ok())
        .collect()
}

/// Mojang's id for a Forge Minecraft key. Forge writes `1.7.10_pre4` where Mojang writes `1.7.10-pre4`.
fn mojang_id(minecraft: &str) -> String {
    minecraft.replacen("_pre", "-pre", 1)
}

/// What a Minecraft version needs before its Forge install runs: the Java that runs the installer, and the Mojang version as
/// Mojang gives it. The launcher saves that version, with its client jar and libraries, into its store before the loader stage,
/// and the Forge processors read some of them from there.
#[derive(Debug, Clone)]
struct Setup {
    requirement: java::Requirement,
    program: PathBuf,
    version_json: String,
    manifest: VersionManifest,
}

/// Reads each installable version's Mojang version, then installs the Java runtime each one needs, once per component. A version
/// whose JSON or runtime cannot be set up carries that error, and its row fails with it.
async fn prepare(client: &reqwest::Client, cases: &[Case], root: &Path) -> BTreeMap<String, Result<Setup, String>> {
    let mojang = lodestone_loaders::mojang::version_list(client).await;
    let mut versions: BTreeMap<String, Result<(String, VersionManifest), String>> = BTreeMap::new();
    for case in cases.iter().filter(|case| case.gap.is_none()) {
        let id = mojang_id(&case.minecraft);
        let fetched = match &mojang {
            Err(error) => Err(format!("could not read Mojang's version list: {error}")),
            Ok(list) => match list.versions.iter().find(|version| version.id == id) {
                None => Err(format!("Mojang's version list has no {id}")),
                Some(version) => fetch_version(client, &version.url).await,
            },
        };
        versions.insert(case.minecraft.clone(), fetched);
    }

    let list = java::fetch_list(client).await;
    let mut runtimes: BTreeMap<String, Result<PathBuf, String>> = BTreeMap::new();
    for (_, manifest) in versions.values().flatten() {
        let component = java::Requirement::from_mojang(manifest.java_version.as_ref()).component;
        if runtimes.contains_key(&component) {
            continue;
        }
        let installed = match &list {
            Ok(list) => java::install_runtime(client, list, &component, &root.join("java")).await,
            Err(error) => Err(error.clone()),
        };
        runtimes.insert(component, installed);
    }

    versions
        .into_iter()
        .map(|(minecraft, fetched)| {
            let setup = fetched.and_then(|(version_json, manifest)| {
                let requirement = java::Requirement::from_mojang(manifest.java_version.as_ref());
                let program = runtimes
                    .get(&requirement.component)
                    .cloned()
                    .unwrap_or_else(|| Err(format!("no {} runtime was set up", requirement.component)))?;
                Ok(Setup {
                    requirement,
                    program,
                    version_json,
                    manifest,
                })
            });
            (minecraft, setup)
        })
        .collect()
}

/// A Mojang version exactly as it came, with its manifest.
async fn fetch_version(client: &reqwest::Client, url: &str) -> Result<(String, VersionManifest), String> {
    let text = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| format!("could not fetch {url}: {error}"))?
        .text()
        .await
        .map_err(|error| format!("could not read {url}: {error}"))?;
    let manifest: VersionManifest = serde_json::from_str(&text).map_err(|error| format!("{url} is not a version JSON: {error}"))?;
    Ok((text, manifest))
}

/// Writes a Mojang version JSON where the launcher keeps it, `versions/<id>/<id>.json`, so the installer finds it there.
fn write_version_json(dir: &Path, id: &str, text: &str) -> Result<(), String> {
    let folder = dir.join("versions").join(id);
    std::fs::create_dir_all(&folder).map_err(|error| format!("could not create {}: {error}", folder.display()))?;
    std::fs::write(folder.join(format!("{id}.json")), text).map_err(|error| format!("could not write {id}.json: {error}"))
}

/// The client jar and the libraries the launcher's download stage leaves in the shared store before the loader stage. Forge's
/// processors read some of them from there, so the install needs them in place.
async fn vanilla_files(client: &reqwest::Client, dir: &Path, manifest: &VersionManifest) -> Result<(), String> {
    let jar = dir.join("versions").join(&manifest.id).join(format!("{}.jar", manifest.id));
    manifest
        .download_client(&jar, true, None)
        .await
        .map_err(|error| format!("the client jar: {error}"))?;
    let libraries: Vec<LibraryItem> = manifest
        .libraries
        .iter()
        .filter(|library| rules_allow(library.rules.as_deref()))
        .cloned()
        .collect();
    libraries
        .download_with_client(client, dir.join("libraries"), LIBRARY_CONCURRENCY, None)
        .await
        .map_err(|error| format!("the libraries: {error}"))?;
    Ok(())
}

/// Whether a library's rules let it into an install on this platform, as the launcher reads them. A library with no rules is
/// always wanted. Otherwise the last rule that applies to this OS decides, and a rule that names features never applies, since
/// no feature is switched on here.
fn rules_allow(rules: Option<&[Rule]>) -> bool {
    let Some(rules) = rules else {
        return true;
    };
    let os = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    };
    let mut allowed = false;
    for rule in rules.iter().filter(|rule| rule.features.is_none()) {
        let applies = rule
            .os
            .as_ref()
            .is_none_or(|rule_os| rule_os.name.as_deref().is_none_or(|name| name == os));
        if applies {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

/// The installer's output lines, written to a file on a task of their own. The installer's pipe is read into the channel, so
/// the channel has to keep draining, or the installer waits on it.
struct InstallerLog {
    sender: Sender<String>,
    task: JoinHandle<()>,
}

impl InstallerLog {
    fn start(path: PathBuf) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<String>(256);
        let task = tokio::spawn(async move {
            let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path).ok();
            while let Some(line) = receiver.recv().await {
                if let Some(file) = file.as_mut() {
                    let _ = writeln!(file, "{line}");
                }
            }
        });
        Self { sender, task }
    }

    /// Stops accepting lines, and returns once the file holds every line.
    async fn finish(self) {
        drop(self.sender);
        let _ = self.task.await;
    }
}

/// Runs one version. A known gap expects its 404, and any other version downloads, installs and checks its Forge build.
async fn run_case(
    loader: &Loader<Forge>,
    client: &reqwest::Client,
    root: &Path,
    case: &Case,
    setups: &BTreeMap<String, Result<Setup, String>>,
) -> Row {
    let started = Instant::now();
    let dir = root.join(&case.minecraft);
    let mut row = match case.gap {
        Some(reason) => run_gap(loader, &dir, case, reason).await,
        None => {
            let setup = setups
                .get(&case.minecraft)
                .cloned()
                .unwrap_or_else(|| Err("no setup was prepared".to_string()));
            run_install(loader, client, &dir, case, setup).await
        }
    };
    row.seconds = started.elapsed().as_secs();
    row
}

/// A known gap: its download must fail with a 404 on the installer. Anything else is a problem.
async fn run_gap(loader: &Loader<Forge>, dir: &Path, case: &Case, reason: &str) -> Row {
    let mut row = Row::new(&case.minecraft, &case.version.version);
    let result = loader.download_client(&case.version, dir.join("libraries"), None).await;
    let _ = std::fs::remove_dir_all(dir);
    match result {
        Err(LoaderError::Download { url, source }) if url.ends_with("-installer.jar") && is_not_found(source.as_ref()) => {
            row.download = format!("404 on the installer ({reason})");
            row.outcome = Outcome::KnownGap;
            row.note = reason.to_string();
        }
        Ok(_) => {
            row.download = "downloads".to_string();
            row.outcome = Outcome::NowInstalls;
            row.note = format!("it downloads now ({reason} before); remove it from KNOWN_GAPS");
        }
        Err(error) => {
            row.download = "failed".to_string();
            row.outcome = Outcome::Fail;
            row.note = format!("expected a 404 on the installer, got {}", note_of(&error));
        }
    }
    row
}

/// Whether a download failed because the server answered 404.
fn is_not_found(source: &(dyn std::error::Error + Send + Sync + 'static)) -> bool {
    matches!(
        source.downcast_ref::<lodestone_net::download::Error>(),
        Some(lodestone_net::download::Error::Status { status: 404, .. })
    )
}

/// Runs one installable version. A passing version's folder is removed. A failing one is kept, and its row names the step that
/// failed and the installer log that holds its output.
async fn run_install(loader: &Loader<Forge>, client: &reqwest::Client, dir: &Path, case: &Case, setup: Result<Setup, String>) -> Row {
    let mut row = Row::new(&case.minecraft, &case.version.version);
    let _ = std::fs::remove_dir_all(dir);
    match install_steps(loader, client, dir, case, setup, &mut row).await {
        Ok(()) => {
            row.outcome = Outcome::Pass;
            let _ = std::fs::remove_dir_all(dir);
        }
        Err(problem) => {
            row.outcome = Outcome::Fail;
            row.note = format!("{problem} (log: {})", dir.join("installer.log").display());
        }
    }
    row
}

/// The steps of one installable version, in order. The first step that fails ends the run, and its message is returned. Each
/// step fills its cell of the row as it runs.
async fn install_steps(
    loader: &Loader<Forge>,
    client: &reqwest::Client,
    dir: &Path,
    case: &Case,
    setup: Result<Setup, String>,
    row: &mut Row,
) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| format!("could not create {}: {error}", dir.display()))?;
    let setup = setup.map_err(|error| {
        row.java = "not set up".to_string();
        format!("setup: {error}")
    })?;
    row.java = format!("{} (Java {})", setup.requirement.component, setup.requirement.major);

    // The launcher's download stage fills the shared store before the loader stage runs, so the same files go in first.
    write_version_json(dir, &mojang_id(&case.minecraft), &setup.version_json).map_err(|error| fail_step(&mut row.vanilla, "vanilla", &error))?;
    vanilla_files(client, dir, &setup.manifest)
        .await
        .map_err(|problem| fail_step(&mut row.vanilla, "vanilla", &problem))?;
    row.vanilla = "ok".to_string();

    let downloads = loader
        .download_client(&case.version, dir.join("libraries"), None)
        .await
        .map_err(|error| fail_step(&mut row.download, "download", &error))?;
    let installer = installer_of(&downloads).ok_or_else(|| {
        row.download = "failed".to_string();
        "download: no installer jar was downloaded".to_string()
    })?;
    let universal = downloads.iter().any(|path| file_name_ends_with(path, "-universal.jar"));
    row.download = if universal { "installer + universal" } else { "installer only" }.to_string();
    row.layout = verify::installer_layout(&installer);

    let log = InstallerLog::start(dir.join("installer.log"));
    let result = install_and_check(loader, dir, case, &setup, &log, row).await;
    log.finish().await;
    result
}

/// The install of one version and its checks. The install is retried once on the system Java when its outputs were rejected,
/// as the launcher does. Then what it wrote is verified, and a second install must reuse it.
async fn install_and_check(loader: &Loader<Forge>, dir: &Path, case: &Case, setup: &Setup, log: &InstallerLog, row: &mut Row) -> Result<(), String> {
    let installed: InstalledVersion = match loader.install(&case.version, dir, &setup.program, Some(&log.sender), None).await {
        Ok(installed) => {
            row.install = "ok".to_string();
            installed
        }
        Err(error) if error.is_output_mismatch() => {
            let Some(system) = java::system_java(setup.requirement.major).await else {
                row.install = "invalid outputs; no system Java to retry with".to_string();
                return Err(format!("install: {}", note_of(&error)));
            };
            let _ = log
                .sender
                .send(format!("retrying on system Java {} after invalid outputs", system.major))
                .await;
            match loader.install(&case.version, dir, &system.program, Some(&log.sender), None).await {
                Ok(installed) => {
                    row.install = format!("retried on system Java {}", system.major);
                    installed
                }
                Err(error) => {
                    row.install = "failed on the retry".to_string();
                    return Err(format!("install (retry): {}", note_of(&error)));
                }
            }
        }
        Err(error) => return Err(fail_step(&mut row.install, "install", &error)),
    };

    // A `versionInfo` installer's profile is written as it is, and may name no parent version.
    let legacy = row.layout.starts_with("versionInfo");
    verify::installed(dir, &case.minecraft, &installed, legacy).map_err(|problem| fail_step(&mut row.verify, "verify", &problem))?;
    row.verify = "ok".to_string();

    match loader.install(&case.version, dir, &setup.program, None, None).await {
        Ok(again) if again.id == installed.id => row.reinstall = "ok".to_string(),
        Ok(again) => {
            row.reinstall = "different version".to_string();
            return Err(format!("reinstall wrote {} where the first install wrote {}", again.id, installed.id));
        }
        Err(error) => return Err(fail_step(&mut row.reinstall, "reinstall", &error)),
    }
    Ok(())
}

/// The installer jar among the files a download returned.
fn installer_of(paths: &[PathBuf]) -> Option<PathBuf> {
    paths.iter().find(|path| file_name_ends_with(path, "-installer.jar")).cloned()
}

fn file_name_ends_with(path: &Path, suffix: &str) -> bool {
    path.file_name().is_some_and(|name| name.to_string_lossy().ends_with(suffix))
}

/// Records a failed step in its cell of the row, and returns the message that names the step.
fn fail_step(cell: &mut String, step: &str, error: &impl Display) -> String {
    *cell = "failed".to_string();
    format!("{step}: {}", note_of(error))
}

/// A short note for a failure: the first line of the error, and its last line when there is more than one. For an installer
/// the last line is its final output line.
fn note_of(error: &impl Display) -> String {
    let text = error.to_string();
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
    match (lines.first(), lines.last()) {
        (Some(first), Some(last)) if lines.len() > 1 => format!("{first} … {last}"),
        (Some(only), _) => (*only).to_string(),
        _ => "no message".to_string(),
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "installs every Minecraft version's latest Forge build with real installers: run it with `just forge-matrix`"]
async fn every_forge_version_installs() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("forge-install");
    let jobs = std::env::var(JOBS_VAR)
        .ok()
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(DEFAULT_JOBS)
        .max(1);
    let only: Option<HashSet<String>> = std::env::var(VERSIONS_VAR).ok().map(|value| {
        value
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect()
    });

    let client = lodestone_net::client();
    let loader = Loader::<Forge>::new();
    let all = loader.list_versions().await.expect("Forge's version index loads");
    let cases = select(&all, only.as_ref()).unwrap_or_else(|error| panic!("{error}"));
    let setups = prepare(&client, &cases, &root).await;

    let mut rows: Vec<Row> = futures_util::stream::iter(&cases)
        .map(|case| run_case(&loader, &client, &root, case, &setups))
        .buffer_unordered(jobs)
        .collect()
        .await;
    rows.sort_by_key(|row| version_key(&row.minecraft));

    let report_path = root.join("report.md");
    report::write_and_print(&rows, &report_path);
    let problems: Vec<String> = rows
        .iter()
        .filter(|row| row.outcome.is_problem())
        .map(|row| format!("{}: {}", row.minecraft, row.note))
        .collect();
    assert!(
        problems.is_empty(),
        "{} Forge versions did not install as expected (table: {}):\n{}",
        problems.len(),
        report_path.display(),
        problems.join("\n")
    );
}

/// A version as the index lists it, with its Minecraft version bound.
fn build(version: &str, minecraft: &str) -> LoaderVersion {
    LoaderVersion {
        version: version.to_string(),
        minecraft: Some(minecraft.to_string()),
        stable: true,
        changelog_url: None,
    }
}

#[test]
fn the_latest_build_is_the_last_one_the_index_lists() {
    let all = [
        build("1.20.1-47.4.0", "1.20.1"),
        build("1.7.10-10.13.0.1150", "1.7.10"),
        build("1.20.1-47.4.29", "1.20.1"),
        build("1.7.10-10.13.4.1614-1.7.10", "1.7.10"),
    ];
    let cases = select(&all, None).expect("the selection builds");
    let picked: Vec<(&str, &str)> = cases
        .iter()
        .map(|case| (case.minecraft.as_str(), case.version.version.as_str()))
        .collect();
    assert_eq!(picked, [("1.7.10", "1.7.10-10.13.4.1614-1.7.10"), ("1.20.1", "1.20.1-47.4.29")]);
}

#[test]
fn a_selection_narrows_to_the_named_versions_and_rejects_unknown_ones() {
    let all = [build("1.20.1-47.4.29", "1.20.1"), build("1.21.1-52.1.21", "1.21.1")];
    let only: HashSet<String> = ["1.21.1".to_string()].into_iter().collect();
    let cases = select(&all, Some(&only)).expect("the narrowed selection builds");
    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].minecraft, "1.21.1");

    let unknown: HashSet<String> = ["9.9".to_string()].into_iter().collect();
    assert!(select(&all, Some(&unknown)).is_err());
}

#[test]
fn a_known_gap_is_marked_on_its_case() {
    let all = [build("1.5.1-7.7.2.682", "1.5.1"), build("1.20.1-47.4.29", "1.20.1")];
    let cases = select(&all, None).expect("the selection builds");
    let gaps: Vec<&str> = cases
        .iter()
        .filter(|case| case.gap.is_some())
        .map(|case| case.minecraft.as_str())
        .collect();
    assert_eq!(gaps, ["1.5.1"]);
}

#[test]
fn version_keys_sort_by_their_numbers() {
    let mut versions = ["1.10", "1.9", "1.20.1", "26.1", "1.7.10_pre4", "1.7.10"];
    versions.sort_by_key(|version| version_key(version));
    assert_eq!(versions, ["1.7.10", "1.7.10_pre4", "1.9", "1.10", "1.20.1", "26.1"]);
}

#[test]
fn a_forge_prerelease_key_names_mojangs_version() {
    assert_eq!(mojang_id("1.7.10_pre4"), "1.7.10-pre4");
    assert_eq!(mojang_id("1.20.1"), "1.20.1");
}

#[test]
fn a_version_without_a_java_version_runs_on_java_8() {
    let legacy = java::Requirement::from_mojang(None);
    assert_eq!(legacy.component, "jre-legacy");
    assert_eq!(legacy.major, 8);
}

#[test]
fn the_java_major_is_read_from_the_version_line() {
    assert_eq!(
        java::parse_major("openjdk version \"17.0.20.1\" 2026-08-18\nOpenJDK Runtime Environment"),
        Some(17)
    );
    assert_eq!(java::parse_major("java version \"1.8.0_392\"\nJava(TM) SE Runtime Environment"), Some(8));
    assert_eq!(
        java::parse_major("Picked up JAVA_TOOL_OPTIONS: -Dfoo=bar\nopenjdk version \"21.0.5\" 2024-10-15"),
        Some(21)
    );
    assert_eq!(java::parse_major("no version line here"), None);
}

#[test]
fn each_known_gap_names_a_version_once() {
    let mut names: Vec<&str> = KNOWN_GAPS.iter().map(|(name, _)| *name).collect();
    names.sort_unstable();
    let total = names.len();
    names.dedup();
    assert_eq!(names.len(), total);
}

/// An empty folder for one offline test, under the target's temporary folder.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("forge-install-offline").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the scratch folder creates");
    dir
}

/// A finished install of Forge 1.20.1 under `game`: its version JSON, and the Forge library the JSON names when `library` is
/// set.
fn write_install(game: &Path, library: bool) -> InstalledVersion {
    let id = "1.20.1-forge-47.4.29";
    let json_path = game.join("versions").join(id).join(format!("{id}.json"));
    std::fs::create_dir_all(json_path.parent().expect("the version JSON has a folder")).expect("the version folder creates");
    let json = serde_json::json!({
        "id": id,
        "inheritsFrom": "1.20.1",
        "mainClass": "cpw.mods.bootstraplauncher.BootstrapLauncher",
        "libraries": [
            { "name": "net.minecraftforge:forge:1.20.1-47.4.29:universal" },
            { "name": "org.ow2.asm:asm:9.5" }
        ]
    });
    std::fs::write(&json_path, json.to_string()).expect("the version JSON writes");
    if library {
        let jar = game.join("libraries/net/minecraftforge/forge/1.20.1-47.4.29/forge-1.20.1-47.4.29-universal.jar");
        std::fs::create_dir_all(jar.parent().expect("the library has a folder")).expect("the library folder creates");
        std::fs::write(&jar, b"jar").expect("the library writes");
    }
    InstalledVersion {
        id: id.to_string(),
        json: json_path,
    }
}

#[test]
fn a_complete_install_verifies() {
    let game = scratch("verify-complete");
    let installed = write_install(&game, true);
    assert_eq!(verify::installed(&game, "1.20.1", &installed, false), Ok(()));
}

#[test]
fn a_missing_forge_library_fails_verification() {
    let game = scratch("verify-missing-library");
    let installed = write_install(&game, false);
    let problem = verify::installed(&game, "1.20.1", &installed, false).expect_err("the named library is missing");
    assert!(problem.contains("forge-1.20.1-47.4.29-universal.jar"), "{problem}");
}

#[test]
fn a_version_for_another_minecraft_fails_verification() {
    let game = scratch("verify-parent");
    let installed = write_install(&game, true);
    let problem = verify::installed(&game, "1.20.2", &installed, false).expect_err("the version inherits another Minecraft");
    assert!(problem.contains("inherit Minecraft 1.20.2"), "{problem}");
}

#[test]
fn a_version_without_a_main_class_fails_verification() {
    let game = scratch("verify-main-class");
    let installed = write_install(&game, true);
    std::fs::write(&installed.json, r#"{"id":"1.20.1-forge-47.4.29","inheritsFrom":"1.20.1"}"#).expect("the JSON rewrites");
    let problem = verify::installed(&game, "1.20.1", &installed, false).expect_err("the version names no main class");
    assert!(problem.contains("mainClass"), "{problem}");
}

#[test]
fn an_installer_layout_tells_processors_from_the_older_profile() {
    let modern = fixture::jar(
        "forge-layout-modern",
        &[("install_profile.json", r#"{"processors":[]}"#), ("version.json", "{}")],
    );
    assert_eq!(verify::installer_layout(&modern), "processors + version.json");

    let older = fixture::jar("forge-layout-older", &[("install_profile.json", r#"{"versionInfo":{}}"#)]);
    assert_eq!(verify::installer_layout(&older), "versionInfo");

    let bare = fixture::jar("forge-layout-bare", &[("META-INF/MANIFEST.MF", "Manifest-Version: 1.0")]);
    assert_eq!(verify::installer_layout(&bare), "no profile");
}
