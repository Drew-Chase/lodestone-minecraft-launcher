//! Getting an instance ready to start. The stages mirror an install: the shared files its version
//! needs, then its loader, then the libraries and natives of the version the game starts. Files
//! already on disk are checked rather than downloaded again. The result is the launch description
//! the launcher spawns.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use lodestone_launch::Launcher;
use lodestone_server::address::ServerAddress;
use sipper::Sender as ProgressSender;
use sipper::{Sipper, sipper};
use tokio::sync::mpsc;

use super::arguments::{self, Values};
use super::identity::Identity;
use super::libraries::{self, Plan};
use super::manifest::Manifest;
use super::rules::Platform;
use crate::app::install::download::PartReport;
use crate::app::install::{self, Totals, drive, fraction};
use crate::app::lang;
use crate::app::launch_settings::{EnvVar, JavaChoice, LaunchMode, LaunchSettings, split_arguments, valid_env_name};
use crate::app::pages::library::instance::Loader;
use crate::app::paths::LauncherPaths;
use crate::app::{AZURE_CLIENT_ID, Account, LodestoneLauncher};

/// The brand the game reports itself under.
const LAUNCHER_NAME: &str = "Lodestone Launcher";

/// How close to expiry, in seconds, a Microsoft session is refreshed before a launch.
const REFRESH_WITHIN: u64 = 300;

/// What a launch joins as the game starts: a world of the instance, or a server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickPlay {
    /// A world, by its folder name.
    World(String),
    /// A server, by its address.
    Server(ServerAddress),
}

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
    /// What the launch joins as the game starts. A world needs a version that can quick play (see
    /// [`supports_quick_play`]).
    pub quick_play: Option<QuickPlay>,
    /// The launch settings the instance resolves to: its overrides merged with the global defaults.
    pub settings: LaunchSettings,
}

/// The game arguments that join `server` as the game starts. A version that can quick play takes the address as one
/// argument, and an older one takes the host and the port apart.
pub fn join_arguments(mc_version: &str, server: &ServerAddress) -> Vec<String> {
    if supports_quick_play(mc_version) {
        let host = if server.host.contains(':') {
            format!("[{}]", server.host)
        } else {
            server.host.clone()
        };
        vec!["--quickPlayMultiplayer".to_owned(), format!("{host}:{}", server.port)]
    } else {
        vec!["--server".to_owned(), server.host.clone(), "--port".to_owned(), server.port.to_string()]
    }
}

/// Whether a Minecraft version can quick play into a world from the command line. `--quickPlaySingleplayer`
/// arrived with 1.20, so an older version, a snapshot, or anything that does not read as `1.N[.P]` starts
/// the instance alone.
pub fn supports_quick_play(mc_version: &str) -> bool {
    let mut parts = mc_version.split('.');
    let (Some(major), Some(minor)) = (parts.next(), parts.next()) else {
        return false;
    };
    major == "1" && minor.parse::<u32>().is_ok_and(|minor| minor >= 20)
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
    let client = lodestone_net::client();
    let identity = identity_for(&account);
    let mc = target.mc_version.as_str();
    let settings = &target.settings;
    let verify = !settings.skip_integrity;

    let vanilla = {
        let _version = install::ensure::lock(format!("version:{mc}")).await;
        install::ensure::vanilla(&paths, &client, mc).await?
    };
    let runtime = runtime_of(&settings.java, &vanilla);
    let (java, log_config) = download_stage(progress, &paths, &client, &vanilla, &runtime, verify).await?;
    let log_config = match log_config {
        Some(config) if settings.verbose_logging => Some(debug_log_config(config).await?),
        other => other,
    };
    let version_id = match target.loader {
        Loader::Vanilla => mc.to_string(),
        _ => loader_stage(progress, &paths, &target, &java, vanilla.java_major()).await?,
    };

    let merged = merged_manifest(&paths, &version_id, &vanilla)?;
    let plan = libraries::plan(&merged, &paths.libraries(), Platform::current());
    libraries_stage(progress, &client, &vanilla, &plan, verify).await?;
    let natives = paths.natives(&merged.id);
    install::ensure::natives(natives.clone(), plan.natives.clone()).await?;

    let launcher = build(Build {
        paths: &paths,
        target: &target,
        merged: &merged,
        vanilla: &vanilla,
        plan: &plan,
        log_config: log_config.as_deref(),
        identity: &identity,
        java: &java,
        natives,
    })?;
    if !settings.pre_launch_hook.trim().is_empty() {
        run_hook(settings.pre_launch_hook.clone(), target.directory.clone(), settings.env.clone()).await?;
    }
    Ok(Prepared { launcher })
}

/// Where the game's Java program comes from.
enum Runtime {
    /// A Mojang runtime, by component, to install and run.
    Bundled(String),
    /// The Java found on `JAVA_HOME` or `PATH`.
    System,
}

/// The runtime a launch uses. Follow runs the one the version asks for, as launches always have.
fn runtime_of(choice: &JavaChoice, vanilla: &Manifest) -> Runtime {
    match choice {
        JavaChoice::Follow => Runtime::Bundled(
            vanilla
                .java_version
                .as_ref()
                .map_or_else(|| "jre-legacy".to_string(), |java| java.component.clone()),
        ),
        JavaChoice::Bundled { component } => Runtime::Bundled(component.clone()),
        JavaChoice::System => Runtime::System,
    }
}

/// Makes sure the version's shared files are on disk, the parts running side by side, and returns the Java
/// program the game runs with and the log config the version names.
async fn download_stage(
    progress: &mut ProgressSender<Progress>,
    paths: &LauncherPaths,
    client: &reqwest::Client,
    vanilla: &Manifest,
    runtime: &Runtime,
    verify: bool,
) -> Result<(PathBuf, Option<PathBuf>), String> {
    let (reports_tx, reports) = mpsc::channel::<PartReport>(256);
    let mut totals = Totals::default();
    let work = async {
        match runtime {
            Runtime::Bundled(component) => install::download(paths, client, vanilla, component, &reports_tx, verify)
                .await
                .map(|downloaded| (downloaded.java.0, downloaded.log_config)),
            Runtime::System => match system_java(vanilla.java_major()).await {
                Ok(java) => install::shared_files(paths, client, vanilla, &reports_tx, verify)
                    .await
                    .map(|log_config| (java, log_config)),
                Err(error) => Err(error),
            },
        }
    };
    drive(progress, reports, move |report| file_progress(&mut totals, report), work).await
}

/// The Java program on the system, which must be at least `at_least` major.
async fn system_java(at_least: u32) -> Result<PathBuf, String> {
    install::java::system(at_least)
        .await
        .map(|found| found.program)
        .ok_or_else(|| lang::t_with("launch.no_system_java", &[("major", at_least.to_string().as_str())]))
}

/// Installs the loader unless its version is already in the store, and returns the id of the
/// version the game starts.
async fn loader_stage(
    progress: &mut ProgressSender<Progress>,
    paths: &LauncherPaths,
    target: &Target,
    java: &Path,
    game_major: u32,
) -> Result<String, String> {
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
    let work = install::loader::install(
        &shared,
        &target.mc_version,
        target.loader,
        loader_version,
        java,
        game_major,
        Some(&lines_tx),
    );
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
async fn libraries_stage(
    progress: &mut ProgressSender<Progress>,
    client: &reqwest::Client,
    vanilla: &Manifest,
    plan: &Plan,
    verify: bool,
) -> Result<(), String> {
    let _version = install::ensure::lock(format!("version:{}", vanilla.id)).await;
    let (reports_tx, reports) = mpsc::channel::<PartReport>(256);
    let mut totals = Totals::default();
    let work = install::ensure::libraries(client, plan, &reports_tx, verify);
    drive(progress, reports, move |report| file_progress(&mut totals, report), work).await
}

/// One part's report as progress.
fn file_progress(totals: &mut Totals, report: PartReport) -> Progress {
    let sum = totals.apply(report);
    Progress {
        fraction: fraction(sum.done, sum.total),
        detail: lang::t_with(
            "install.files_count",
            &[("done", sum.done.to_string().as_str()), ("total", sum.total.to_string().as_str())],
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

/// Whether the launch is Forge's module bootstrap, which loads Minecraft from the processed jar its installer writes.
fn forge_bootstrap(target: &Target, merged: &Manifest) -> bool {
    target.loader == Loader::Forge
        && matches!(
            merged.main_class.as_deref(),
            Some("cpw.mods.bootstraplauncher.BootstrapLauncher" | "net.minecraftforge.bootstrap.ForgeBootstrap")
        )
}

/// The launcher that spawns the game: the Java program, the classpath, the arguments and the
/// directories the game reads. The instance's launch settings shape the JVM and game arguments, the window and
/// the environment.
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
    let settings = &target.settings;
    let platform = Platform::current();
    // The client jar leads, as Modrinth's launch does, so a loader's libraries come after it. Forge's bootstrap loads
    // Minecraft from its own processed jar, and the vanilla jar would clash with that as a second module, so it stays off.
    let mut classpath = Vec::new();
    if !forge_bootstrap(target, merged) {
        classpath.push(paths.client_jar(&vanilla.id));
    }
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

    let mut jvm = vec![format!("-Xmx{}M", settings.memory_mb), settings.collector.flag().to_string()];
    jvm.extend(arguments::jvm(merged, &values, identity, platform));
    jvm.extend(log_config.and_then(|config| arguments::logging(merged, config)));
    jvm.extend(split_arguments(&settings.jvm_args));
    let mut game = arguments::game(merged, &values, identity, platform);
    // Quick play opens the world, or joins the server, at once. Minecraft finds a world by its folder name.
    match &target.quick_play {
        Some(QuickPlay::World(world)) => {
            game.push("--quickPlaySingleplayer".to_string());
            game.push(world.clone());
        }
        Some(QuickPlay::Server(server)) => game.extend(join_arguments(&target.mc_version, server)),
        None => {}
    }
    game.extend(window_arguments(settings));
    game.extend(split_arguments(&settings.game_args));
    let main_class = merged.main_class.clone().ok_or_else(|| lang::t("launch.no_main_class").to_owned())?;

    Launcher::builder(java, &target.directory)
        .library_directory(libraries)
        .natives_directory(natives)
        .assets_directory(assets)
        .jvm_args(jvm)
        .main_class(main_class)
        .classpath(classpath)
        .game_args(game)
        .env(
            settings
                .env
                .iter()
                .filter(|var| valid_env_name(&var.name))
                .map(|var| (var.name.clone(), var.value.clone())),
        )
        .launcher_name(LAUNCHER_NAME)
        .launcher_version(LodestoneLauncher::VERSION)
        .build()
        .map_err(|error| error.to_string())
}

/// The arguments that size the window, and make it fullscreen when the launch mode asks for it.
fn window_arguments(settings: &LaunchSettings) -> Vec<String> {
    let mut arguments = vec![
        "--width".to_string(),
        settings.resolution.width.to_string(),
        "--height".to_string(),
        settings.resolution.height.to_string(),
    ];
    if settings.launch_mode == LaunchMode::Fullscreen {
        arguments.push("--fullscreen".to_string());
    }
    arguments
}

/// The version's log config, copied with its root logger at DEBUG beside the original. The copy is what the
/// game is given when verbose logging is on.
async fn debug_log_config(config: PathBuf) -> Result<PathBuf, String> {
    tokio::task::spawn_blocking(move || {
        let name = config
            .file_name()
            .map_or_else(|| "log4j.xml".to_string(), |name| name.to_string_lossy().into_owned());
        let path = config.with_file_name(format!("debug-{name}"));
        let text = std::fs::read_to_string(&config).map_err(|error| error.to_string())?;
        std::fs::write(&path, debug_log_text(&text)).map_err(|error| error.to_string())?;
        Ok::<PathBuf, String>(path)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| lang::t_with("launch.log_config_failed", &[("error", error.as_str())]))
}

/// `text` with its root logger at DEBUG. A config that does not name INFO at the root comes back unchanged.
fn debug_log_text(text: &str) -> String {
    text.replace("<Root level=\"info\">", "<Root level=\"debug\">")
}

/// Runs the pre-launch hook through the shell, in the instance folder with the instance's variables, and waits
/// for it. A hook that exits with an error stops the launch.
async fn run_hook(command: String, directory: PathBuf, env: Vec<EnvVar>) -> Result<(), String> {
    let status = tokio::task::spawn_blocking(move || {
        let mut shell = if cfg!(windows) {
            let mut shell = std::process::Command::new("cmd");
            shell.arg("/C");
            shell
        } else {
            let mut shell = std::process::Command::new("sh");
            shell.arg("-c");
            shell
        };
        shell
            .arg(&command)
            .current_dir(&directory)
            .envs(env.iter().filter(|var| valid_env_name(&var.name)).map(|var| (&var.name, &var.value)))
            .status()
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| lang::t_with("launch.hook_failed", &[("error", error.to_string().as_str())]))?;
    if status.success() {
        return Ok(());
    }
    let code = status
        .code()
        .map_or_else(|| lang::t("launch.hook_no_code").to_owned(), |code| code.to_string());
    Err(lang::t_with("launch.hook_exit", &[("code", code.as_str())]))
}

#[cfg(test)]
#[path = "../../../tests/app/launch/prepare.rs"]
mod tests;
