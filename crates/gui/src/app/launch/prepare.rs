//! Getting an instance ready to start. The stages mirror an install: the shared files its version
//! needs, then its loader, then the libraries and natives of the version the game starts. Files
//! already on disk are checked rather than downloaded again. The result is the launch description
//! the launcher spawns.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use lodestone_launch::Launcher;
use sipper::Sender as ProgressSender;
use sipper::{Sipper, sipper};
use tokio::sync::mpsc;

use super::arguments::{self, Values};
use super::identity::Identity;
use super::libraries::{self, Plan};
use super::manifest::Manifest;
use super::rules::Platform;
use crate::app::install::download::PartReport;
use crate::app::install::{self, Downloaded, Totals, drive, fraction};
use crate::app::lang;
use crate::app::pages::library::instance::Loader;
use crate::app::paths::LauncherPaths;
use crate::app::{AZURE_CLIENT_ID, Account, LodestoneLauncher};

/// The memory the game is given, in megabytes, until instances keep their own setting.
const MEMORY_MB: u32 = 4096;

/// The brand the game reports itself under.
const LAUNCHER_NAME: &str = "Lodestone Launcher";

/// How close to expiry, in seconds, a Microsoft session is refreshed before a launch.
const REFRESH_WITHIN: u64 = 300;

/// The instance a launch starts, as its database row describes it.
#[derive(Debug, Clone)]
pub struct Target {
    pub mc_version: String,
    pub loader: Loader,
    pub loader_version: Option<String>,
    /// The version the instance was installed with, when it was.
    pub version_id: Option<String>,
    /// The instance folder, which is the game's directory.
    pub directory: PathBuf,
}

/// Everything a preparation needs. The account's session must already be fresh: the root refreshes
/// it first (see [`session_check`]), so only one refresh ever spends a refresh token.
pub struct Request {
    pub paths: LauncherPaths,
    pub target: Target,
    pub account: Account,
}

/// One progress report: the fraction of the files on disk, when known, and a detail line.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    pub fraction: Option<f32>,
    pub detail: String,
}

/// A ready launch: the launcher to spawn.
#[derive(Debug, Clone)]
pub struct Prepared {
    pub launcher: Launcher,
}

/// Prepares the launch described by `request`, reporting its progress as it goes.
pub fn prepare(request: Request) -> impl Sipper<Result<Prepared, String>, Progress> {
    sipper(move |mut progress: ProgressSender<Progress>| async move { run(request, &mut progress).await })
}

/// The stages, in order.
async fn run(request: Request, progress: &mut ProgressSender<Progress>) -> Result<Prepared, String> {
    let Request { paths, target, account } = request;
    let client = reqwest::Client::new();
    let identity = identity_for(&account);
    let mc = target.mc_version.as_str();

    let vanilla = {
        let _version = install::ensure::lock(format!("version:{mc}")).await;
        install::ensure::vanilla(&paths, &client, mc).await?
    };
    let downloaded = download_stage(progress, &paths, &client, &vanilla).await?;
    let (java, _) = &downloaded.java;
    let version_id = match target.loader {
        Loader::Vanilla => mc.to_string(),
        _ => loader_stage(progress, &paths, &target, java).await?,
    };

    let merged = merged_manifest(&paths, &version_id, &vanilla)?;
    let plan = libraries::plan(&merged, &paths.libraries(), Platform::current());
    libraries_stage(progress, &client, &vanilla, &plan).await?;
    let natives = paths.natives(&merged.id);
    install::ensure::natives(natives.clone(), plan.natives.clone()).await?;

    let launcher = build(Build {
        paths: &paths,
        target: &target,
        merged: &merged,
        vanilla: &vanilla,
        plan: &plan,
        log_config: downloaded.log_config.as_deref(),
        identity: &identity,
        java,
        natives,
    })?;
    Ok(Prepared { launcher })
}

/// Makes sure the version's shared files are on disk, the parts running side by side.
async fn download_stage(
    progress: &mut ProgressSender<Progress>,
    paths: &LauncherPaths,
    client: &reqwest::Client,
    vanilla: &Manifest,
) -> Result<Downloaded, String> {
    let component = vanilla
        .java_version
        .as_ref()
        .map_or_else(|| "jre-legacy".to_string(), |java| java.component.clone());
    let (reports_tx, reports) = mpsc::channel::<PartReport>(256);
    let mut totals = Totals::default();
    let work = install::download(paths, client, vanilla, &component, &reports_tx);
    drive(progress, reports, move |report| file_progress(&mut totals, report), work).await
}

/// Installs the loader unless its version is already in the store, and returns the id of the
/// version the game starts.
async fn loader_stage(progress: &mut ProgressSender<Progress>, paths: &LauncherPaths, target: &Target, java: &Path) -> Result<String, String> {
    if let Some(id) = target.version_id.as_deref()
        && paths.version_json(id).is_file()
    {
        return Ok(id.to_string());
    }
    let Some(loader_version) = target.loader_version.as_deref() else {
        return Err(lang::t("install.loader.choose_version").to_owned());
    };
    let shared = paths.meta();
    let (lines_tx, lines) = mpsc::channel::<String>(256);
    let work = install::loader::install(&shared, &target.mc_version, target.loader, loader_version, java, Some(&lines_tx));
    drive(
        progress,
        lines,
        |line| Progress {
            fraction: None,
            detail: line,
        },
        work,
    )
    .await
}

/// Makes sure every library the game's version lists is on disk.
async fn libraries_stage(progress: &mut ProgressSender<Progress>, client: &reqwest::Client, vanilla: &Manifest, plan: &Plan) -> Result<(), String> {
    let _version = install::ensure::lock(format!("version:{}", vanilla.id)).await;
    let (reports_tx, reports) = mpsc::channel::<PartReport>(256);
    let mut totals = Totals::default();
    let work = install::ensure::libraries(client, plan, &reports_tx);
    drive(progress, reports, move |report| file_progress(&mut totals, report), work).await
}

/// One part's report as progress.
fn file_progress(totals: &mut Totals, report: PartReport) -> Progress {
    let (done, total) = totals.apply(report);
    Progress {
        fraction: fraction(done, total),
        detail: lang::t_with(
            "install.files_count",
            &[("done", done.to_string().as_str()), ("total", total.to_string().as_str())],
        ),
    }
}

/// The version the game starts, with its loader's profile folded over the vanilla version.
fn merged_manifest(paths: &LauncherPaths, version_id: &str, vanilla: &Manifest) -> Result<Manifest, String> {
    if version_id == vanilla.id {
        return Ok(vanilla.clone());
    }
    let text = std::fs::read_to_string(paths.version_json(version_id))
        .map_err(|error| lang::t_with("launch.version_read_failed", &[("id", version_id), ("error", error.to_string().as_str())]))?;
    Ok(Manifest::parse(&text)?.over(vanilla.clone()))
}

/// What a launch with an account needs before it can prepare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCheck {
    /// The session is good for the launch, or the account is offline.
    Fresh,
    /// The Microsoft session is close to expiry. It must be refreshed with this refresh token first.
    Refresh(String),
    /// The Microsoft session is close to expiry and holds no refresh token, so the player must sign in again.
    SignInAgain,
}

/// What a launch with `account` needs at `now`, in seconds since the Unix epoch.
pub fn session_check(account: &Account, now: u64) -> SessionCheck {
    match account {
        Account::Offline { .. } => SessionCheck::Fresh,
        Account::Microsoft { session } if session.expires_at > now + REFRESH_WITHIN => SessionCheck::Fresh,
        Account::Microsoft { session } => match &session.refresh_token {
            Some(token) => SessionCheck::Refresh(token.clone()),
            None => SessionCheck::SignInAgain,
        },
    }
}

/// The player the game starts as, for the account it was launched with.
fn identity_for(account: &Account) -> Identity {
    match account {
        Account::Microsoft { session } => Identity::microsoft(&session.profile.name, &session.profile.id, &session.minecraft_token),
        Account::Offline { username } => Identity::offline(username),
    }
}

/// The current time in whole seconds since the Unix epoch.
pub fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |elapsed| elapsed.as_secs())
}

/// What the launch description is built from.
struct Build<'a> {
    paths: &'a LauncherPaths,
    target: &'a Target,
    /// The version the game starts, with its loader folded in.
    merged: &'a Manifest,
    /// The vanilla version, which supplies the client jar and the asset index.
    vanilla: &'a Manifest,
    plan: &'a Plan,
    log_config: Option<&'a Path>,
    identity: &'a Identity,
    java: &'a Path,
    natives: PathBuf,
}

/// The launcher that spawns the game: the Java program, the classpath, the arguments and the
/// directories the game reads.
fn build(parts: Build<'_>) -> Result<Launcher, String> {
    let Build {
        paths,
        target,
        merged,
        vanilla,
        plan,
        log_config,
        identity,
        java,
        natives,
    } = parts;
    let platform = Platform::current();
    // The client jar leads, as Modrinth's launch does, so a loader's libraries come after it.
    let mut classpath = vec![paths.client_jar(&vanilla.id)];
    classpath.extend(plan.classpath.iter().cloned());
    let classpath_text = classpath
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(&platform.classpath_separator().to_string());

    let libraries = paths.libraries();
    let assets = paths.assets();
    let assets_index = vanilla.asset_index.as_ref().map_or_else(String::new, |index| index.id.clone());
    let version_type = merged.kind.clone().unwrap_or_else(|| "release".to_string());
    let values = Values {
        version_name: merged.id.as_str(),
        version_type: version_type.as_str(),
        game_directory: &target.directory,
        assets_root: &assets,
        assets_index_name: assets_index.as_str(),
        library_directory: &libraries,
        natives_directory: &natives,
        classpath: classpath_text.as_str(),
        launcher_name: LAUNCHER_NAME,
        launcher_version: LodestoneLauncher::VERSION,
        client_id: AZURE_CLIENT_ID,
    };

    let mut jvm = vec![format!("-Xmx{MEMORY_MB}M")];
    jvm.extend(arguments::jvm(merged, &values, identity, platform));
    jvm.extend(log_config.and_then(|config| arguments::logging(merged, config)));
    let game = arguments::game(merged, &values, identity, platform);
    let main_class = merged.main_class.clone().ok_or_else(|| lang::t("launch.no_main_class").to_owned())?;

    Launcher::builder(java, &target.directory)
        .library_directory(libraries)
        .natives_directory(natives)
        .assets_directory(assets)
        .jvm_args(jvm)
        .main_class(main_class)
        .classpath(classpath)
        .game_args(game)
        .launcher_name(LAUNCHER_NAME)
        .launcher_version(LodestoneLauncher::VERSION)
        .build()
        .map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "../../../tests/app/launch/prepare.rs"]
mod tests;
