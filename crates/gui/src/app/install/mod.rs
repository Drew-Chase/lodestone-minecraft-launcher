//! Creating an instance. The install makes sure the shared files its version needs are on disk
//! (Java, the client jar, the libraries and the assets, side by side) and then the loader. It
//! writes the instance folder and inserts the database row. Files already on disk are checked
//! rather than downloaded again. The install runs as a sipper that reports progress. Whatever it
//! created is removed again when it fails or is dropped, which is what cancelling the modal does.

pub mod catalog;
pub(crate) mod download;
pub(crate) mod ensure;
pub(crate) mod java;
pub(crate) mod loader;

use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Local;
use sipper::Sender as ProgressSender;
use sipper::{Sipper, sipper};
use tokio::sync::mpsc;
use tracing::warn;

use crate::app::database::Database;
use crate::app::lang;
use crate::app::launch::libraries;
use crate::app::launch::manifest::Manifest;
use crate::app::launch::rules::Platform;
use crate::app::pages::library::instance::{Biome, Instance, InstanceIcon, InstanceState, Loader};
use crate::app::paths::LauncherPaths;
use download::PartReport;

/// The marker a finished shared install leaves behind. Without it the install runs again.
const COMPLETE: &str = ".complete";

/// The folder the uploaded art lives in, inside the instance folder.
const ART_DIR: &str = ".lodestone";
const ICON_FILE: &str = "icon.png";
const BANNER_FILE: &str = "banner.png";

/// The folders every instance starts with. The instance page scans the first four.
const FOLDERS: [&str; 5] = ["mods", "saves", "screenshots", "logs", "config"];

/// Where an instance's icon comes from.
#[derive(Debug, Clone)]
pub enum IconSource {
    /// A background preset and a symbol, by catalog id.
    Composed { background: String, symbol: String },
    /// An uploaded square image, already encoded as PNG.
    Uploaded(Vec<u8>),
}

/// What the install needs, gathered from the New Instance form.
#[derive(Debug, Clone)]
pub struct InstallRequest {
    /// Where the instance folder and the shared downloads go.
    pub paths: LauncherPaths,
    /// The display name.
    pub name: String,
    /// The cleaned, unique folder name inside the instances directory.
    pub directory_name: String,
    /// The Minecraft version.
    pub mc_version: String,
    pub loader: Loader,
    /// The loader's version, for a modded instance.
    pub loader_version: Option<String>,
    /// The Mojang runtime component to use, such as `java-runtime-delta`. `None`
    /// follows the Minecraft version.
    pub java_component: Option<String>,
    pub icon: IconSource,
    /// The scene the banner shows when no banner is uploaded.
    pub biome: Biome,
    /// The scene variant for [`Self::biome`].
    pub art_seed: u64,
    /// An uploaded banner, already encoded as PNG.
    pub banner: Option<Vec<u8>>,
}

/// The stages an install goes through, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Resolve,
    Download,
    Loader,
    Finish,
}

impl Stage {
    /// The line the install dialog shows while this stage runs.
    pub fn label(self) -> &'static str {
        match self {
            Stage::Resolve => lang::t("install.stage.resolve"),
            Stage::Download => lang::t("install.stage.download"),
            Stage::Loader => lang::t("install.stage.loader"),
            Stage::Finish => lang::t("install.stage.finish"),
        }
    }
}

/// One progress report: the stage, its fraction when known, and a detail line.
#[derive(Debug, Clone, PartialEq)]
pub struct InstallProgress {
    pub stage: Stage,
    pub fraction: Option<f32>,
    pub detail: String,
}

/// Runs the install. The sipper reports progress and resolves to the new instance, or to
/// the reason the install stopped. Dropping it removes the instance folder it made.
pub fn install(request: InstallRequest, db: Database) -> impl Sipper<Result<Instance, String>, InstallProgress> {
    sipper(move |mut progress: ProgressSender<InstallProgress>| async move {
        let mut folder = Folder::create(&request.paths.instance(&request.directory_name))?;
        let instance = run(&request, &mut progress, &folder.path, &db).await?;
        folder.keep();
        Ok(instance)
    })
}

/// The message for a folder the install could not create.
fn could_not_create(path: &Path, error: impl std::fmt::Display) -> String {
    path_failed("install.could_not_create", path, error)
}

/// The message for an install step that failed on `path`, from the catalogue key `key`.
fn path_failed(key: &'static str, path: &Path, error: impl std::fmt::Display) -> String {
    lang::t_with(
        key,
        &[("path", path.display().to_string().as_str()), ("error", error.to_string().as_str())],
    )
}

/// The instance folder an install created. Unless it is kept, dropping it deletes the
/// folder, so a failed or cancelled install leaves nothing behind.
struct Folder {
    path: PathBuf,
    kept: bool,
}

impl Folder {
    /// Creates `path`, failing when something already has that name. Making the folder
    /// here is what lets the install delete it later without touching anything else.
    fn create(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| could_not_create(parent, error))?;
        }
        std::fs::create_dir(path).map_err(|error| could_not_create(path, error))?;
        Ok(Self {
            path: path.to_path_buf(),
            kept: false,
        })
    }

    fn keep(&mut self) {
        self.kept = true;
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        if self.kept {
            return;
        }
        let path = std::mem::take(&mut self.path);
        // A loader installer that was stopped may still write for a moment, so the
        // removal waits for it to exit.
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(2));
            if let Err(error) = std::fs::remove_dir_all(&path) {
                warn!("could not remove the partial instance {}: {error}", path.display());
            }
        });
    }
}

/// The stages, in order, writing the instance into `game_dir`.
async fn run(request: &InstallRequest, progress: &mut ProgressSender<InstallProgress>, game_dir: &Path, db: &Database) -> Result<Instance, String> {
    let mc = request.mc_version.as_str();
    let paths = &request.paths;
    let client = reqwest::Client::new();
    report(progress, Stage::Resolve, None, format!("Minecraft {mc}")).await;
    let vanilla = {
        let _version = ensure::lock(format!("version:{mc}")).await;
        ensure::vanilla(paths, &client, mc).await?
    };

    let component = request.java_component.clone().unwrap_or_else(|| {
        vanilla
            .java_version
            .as_ref()
            .map_or_else(|| "jre-legacy".to_string(), |java| java.component.clone())
    });
    let downloaded = download_stage(progress, paths, &client, &vanilla, &component).await?;
    let (java_binary, java_name) = downloaded.java;
    let version_id = match request.loader {
        Loader::Vanilla => mc.to_string(),
        _ => loader_stage(progress, request, &java_binary).await?,
    };
    finish(request, game_dir, version_id, java_name, progress, db).await
}

/// What the download stage left on disk.
pub(crate) struct Downloaded {
    /// The bundled Java program, and its short folder name.
    pub(crate) java: (PathBuf, String),
    /// The log4j config the version names, when it has one.
    pub(crate) log_config: Option<PathBuf>,
}

/// The download stage: Java, the client jar, the libraries and the assets side by side. The
/// stage reports the total of their files.
async fn download_stage(
    progress: &mut ProgressSender<InstallProgress>,
    paths: &LauncherPaths,
    client: &reqwest::Client,
    vanilla: &Manifest,
    component: &str,
) -> Result<Downloaded, String> {
    let (reports_tx, reports) = mpsc::channel::<PartReport>(256);
    let mut totals = Totals::default();
    let work = download(paths, client, vanilla, component, &reports_tx);
    drive(
        progress,
        reports,
        move |report| {
            let (done, total) = totals.apply(report);
            InstallProgress {
                stage: Stage::Download,
                fraction: fraction(done, total),
                detail: lang::t_with(
                    "install.files_count",
                    &[("done", done.to_string().as_str()), ("total", total.to_string().as_str())],
                ),
            }
        },
        work,
    )
    .await
}

/// Makes sure every shared file of the version is on disk, the parts running side by side. The
/// version's lock is held throughout, so a launch of the same version waits for it.
pub(crate) async fn download(
    paths: &LauncherPaths,
    client: &reqwest::Client,
    vanilla: &Manifest,
    component: &str,
    reports: &mpsc::Sender<PartReport>,
) -> Result<Downloaded, String> {
    let _version = ensure::lock(format!("version:{}", vanilla.id)).await;
    let plan = libraries::plan(vanilla, &paths.libraries(), Platform::current());
    let (java, (), (), (), log_config) = tokio::try_join!(
        java::ensure(paths, component, reports),
        ensure::client_jar(paths, client, vanilla, reports),
        ensure::libraries(client, &plan, reports),
        ensure::assets(paths, client, vanilla, reports),
        ensure::log_config(paths, client, vanilla),
    )?;
    Ok(Downloaded { java, log_config })
}

/// The loader stage: installs the loader into the shared store and returns the version id it
/// writes. The installer's output shows as the stage's detail lines.
async fn loader_stage(progress: &mut ProgressSender<InstallProgress>, request: &InstallRequest, java: &Path) -> Result<String, String> {
    let Some(loader_version) = request.loader_version.as_deref() else {
        return Err(lang::t("install.loader.choose_version").to_owned());
    };
    let shared = request.paths.meta();
    let (lines_tx, lines) = mpsc::channel::<String>(256);
    let work = loader::install(&shared, &request.mc_version, request.loader, loader_version, java, Some(&lines_tx));
    drive(
        progress,
        lines,
        |line| InstallProgress {
            stage: Stage::Loader,
            fraction: None,
            detail: line,
        },
        work,
    )
    .await
}

/// Writes the folders and the uploaded art, then inserts the database row.
async fn finish(
    request: &InstallRequest,
    game_dir: &Path,
    version_id: String,
    java_name: String,
    progress: &mut ProgressSender<InstallProgress>,
    db: &Database,
) -> Result<Instance, String> {
    report(progress, Stage::Finish, None, request.directory_name.clone()).await;
    for folder in FOLDERS {
        let path = game_dir.join(folder);
        std::fs::create_dir_all(&path).map_err(|error| could_not_create(&path, error))?;
    }

    let art_dir = game_dir.join(ART_DIR);
    let art_path = match &request.banner {
        Some(png) => {
            write_art(&art_dir, BANNER_FILE, png)?;
            Some(PathBuf::from(ART_DIR).join(BANNER_FILE))
        }
        None => None,
    };
    let icon = match &request.icon {
        IconSource::Composed { background, symbol } => InstanceIcon::Composed {
            background: background.clone(),
            symbol: symbol.clone(),
        },
        IconSource::Uploaded(png) => {
            write_art(&art_dir, ICON_FILE, png)?;
            InstanceIcon::Custom(PathBuf::from(ART_DIR).join(ICON_FILE))
        }
    };

    let instance = Instance {
        id: new_id(),
        name: request.name.clone(),
        mc_version: request.mc_version.clone(),
        loader: request.loader,
        loader_version: request.loader_version.clone(),
        playtime_seconds: 0,
        mod_count: 0,
        last_played: None,
        installed_at: Local::now(),
        pinned: false,
        state: InstanceState::Idle,
        biome: request.biome,
        art_seed: request.art_seed,
        art_path,
        icon,
        directory_name: request.directory_name.clone(),
        java_component: Some(java_name),
        version_id: Some(version_id),
    };
    db.insert_instance(&instance).await?;
    if let IconSource::Composed { background, symbol } = &request.icon
        && let Err(error) = db.record_recent_icon(background, symbol).await
    {
        warn!("could not record the icon as recent: {error}");
    }
    Ok(instance)
}

/// Writes one piece of uploaded art into `dir`, creating `dir` first.
fn write_art(dir: &Path, file: &str, png: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| could_not_create(dir, error))?;
    let path = dir.join(file);
    std::fs::write(&path, png).map_err(|error| path_failed("install.could_not_write", &path, error))
}

/// Reports one progress update.
async fn report(progress: &mut ProgressSender<InstallProgress>, stage: Stage, fraction: Option<f32>, detail: String) {
    progress.send(InstallProgress { stage, fraction, detail }).await;
}

/// The download parts' latest counts. A stage reports their sum, so the bar moves whichever part
/// is working.
#[derive(Default)]
pub(crate) struct Totals {
    parts: [(usize, usize); 4],
}

impl Totals {
    /// Records the part's count and returns the sum of every part's `(done, total)`.
    pub(crate) fn apply(&mut self, report: PartReport) -> (usize, usize) {
        self.parts[report.part as usize] = (report.done, report.total);
        self.parts
            .iter()
            .fold((0, 0), |(done, total), (part_done, part_total)| (done + part_done, total + part_total))
    }
}

/// Runs `work` while the reports it sends on `reports` are forwarded as progress. Returns what
/// `work` returns.
pub(crate) async fn drive<P, T, R>(
    progress: &mut ProgressSender<P>,
    mut reports: mpsc::Receiver<R>,
    mut to_progress: impl FnMut(R) -> P,
    work: impl Future<Output = T>,
) -> T {
    tokio::pin!(work);
    let mut open = true;
    loop {
        tokio::select! {
            report = reports.recv(), if open => match report {
                Some(report) => progress.send(to_progress(report)).await,
                None => open = false,
            },
            output = &mut work => return output,
        }
    }
}

/// `done` of `total` as a fraction, or `None` while the total is unknown.
pub(crate) fn fraction(done: usize, total: usize) -> Option<f32> {
    (total > 0).then(|| (done as f32 / total as f32).min(1.0))
}

/// A random identifier in the 8-4-4-4-12 shape of a uuid.
fn new_id() -> String {
    let hex = format!("{:032x}", fastrand::u128(..));
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

#[cfg(test)]
#[path = "../../../tests/app/install/mod.rs"]
mod tests;
