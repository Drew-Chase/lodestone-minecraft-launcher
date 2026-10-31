//! The Discover page's side effects: the platform searches, the filter lists,
//! the icons, the shared downloads, the modpack installs and the browser. The
//! page decides what to ask for. This module turns those requests into tasks,
//! and sends the answers back as messages.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use iced::Task;
use iced::widget::scrollable::AbsoluteOffset;
use iced::widget::{Id, operation};

use lodestone_platforms::{
    AtLauncher, ContentType, ContentVersion, Facets, Ftb, Modpack, ModpackFile, Modrinth, Platform, SearchPage, SearchQuery, Technic,
};
use tracing::warn;

use super::LodestoneAppMessage;
use super::LodestoneLauncher;
use super::components::alert::Alert;
use super::downloads::JobState;
use super::install::{self, IconSource, InstallRequest, modpack::Payload};
use super::lang;
use super::pages::discover::{self, CONTENT_ID, Effect, InstallState, Message, Project, SEARCH_ID, Search, Source};
use super::pages::instance::Tab;
use super::pages::library::instance::Loader;
use super::pages::library::new_instance::naming;
use super::paths::LauncherPaths;
use super::preferences::Preferences;
use super::shared_library::{self, SharedRecord};

/// How long the page waits after the last keystroke before it searches.
const DEBOUNCE: Duration = Duration::from_millis(300);
/// How many CurseForge file links are resolved at once when a modpack installs.
const RESOLVE_AT_ONCE: usize = 8;

/// The CurseForge key this build was made with, from `LODESTONE_CURSEFORGE_KEY`
/// when it was compiled. Without one, CurseForge cannot be searched.
pub(crate) fn curseforge_key() -> Option<&'static str> {
    option_env!("LODESTONE_CURSEFORGE_KEY").filter(|key| !key.trim().is_empty())
}

/// The platform the page opens on: the one picked last, when this build can search it. Modrinth otherwise.
pub(crate) fn opening_source() -> Source {
    Preferences::load()
        .discover_source
        .as_deref()
        .and_then(Source::from_key)
        .filter(|source| source.enabled())
        .unwrap_or(Source::Modrinth)
}

/// One client per platform, shared by every request, so connections are reused.
static MODRINTH: LazyLock<Platform<Modrinth>> = LazyLock::new(Platform::<Modrinth>::new);
static CURSEFORGE: LazyLock<Option<Platform<lodestone_platforms::CurseForge>>> =
    LazyLock::new(|| curseforge_key().map(Platform::<lodestone_platforms::CurseForge>::with_api_key));
static ATLAUNCHER: LazyLock<Platform<AtLauncher>> = LazyLock::new(Platform::<AtLauncher>::new);
static FTB: LazyLock<Platform<Ftb>> = LazyLock::new(Platform::<Ftb>::new);
static TECHNIC: LazyLock<Platform<Technic>> = LazyLock::new(Platform::<Technic>::new);

/// The CurseForge handle, or the message that says why there is none.
fn curseforge() -> Result<&'static Platform<lodestone_platforms::CurseForge>, String> {
    CURSEFORGE.as_ref().ok_or_else(|| lang::t("discover.curseforge.no_key").to_string())
}

/// Runs one search on `source`, and times it.
async fn run_search(source: Source, query: SearchQuery) -> (Result<SearchPage, String>, u128) {
    let started = Instant::now();
    let result = match source {
        Source::Modrinth => MODRINTH.search(&query).await.map_err(|error| error.to_string()),
        Source::CurseForge => match curseforge() {
            Ok(platform) => platform.search(&query).await.map_err(|error| error.to_string()),
            Err(error) => Err(error),
        },
        Source::AtLauncher => ATLAUNCHER.search(&query).await.map_err(|error| error.to_string()),
        Source::Ftb => FTB.search(&query).await.map_err(|error| error.to_string()),
        Source::Technic => TECHNIC.search(&query).await.map_err(|error| error.to_string()),
    };
    (result, started.elapsed().as_millis())
}

/// The filters `source` offers for `content`.
async fn load_facets(source: Source, content: ContentType) -> Result<Facets, String> {
    match source {
        Source::Modrinth => MODRINTH.facets(content).await.map_err(|error| error.to_string()),
        Source::CurseForge => match curseforge() {
            Ok(platform) => platform.facets(content).await.map_err(|error| error.to_string()),
            Err(error) => Err(error),
        },
        Source::AtLauncher | Source::Technic | Source::Ftb => Ok(Facets::default()),
    }
}

/// The versions of one project on `source`.
async fn versions_of(source: Source, content: ContentType, project_id: &str) -> Result<Vec<ContentVersion>, String> {
    match source {
        Source::Modrinth => MODRINTH.versions(project_id, content).await.map_err(|error| error.to_string()),
        Source::CurseForge => curseforge()?.versions(project_id, content).await.map_err(|error| error.to_string()),
        Source::AtLauncher => ATLAUNCHER.versions(project_id, content).await.map_err(|error| error.to_string()),
        Source::Ftb => FTB.versions(project_id, content).await.map_err(|error| error.to_string()),
        Source::Technic => TECHNIC.versions(project_id, content).await.map_err(|error| error.to_string()),
    }
}

/// Downloads one version's files into `into`, returning their paths.
async fn download_version(source: Source, version: &ContentVersion, into: &Path) -> Result<Vec<PathBuf>, String> {
    match source {
        Source::Modrinth => MODRINTH.download(version, into, None).await.map_err(|error| error.to_string()),
        Source::CurseForge => curseforge()?.download(version, into, None).await.map_err(|error| error.to_string()),
        Source::Ftb => FTB.download(version, into, None).await.map_err(|error| error.to_string()),
        Source::AtLauncher => ATLAUNCHER.download(version, into, None).await.map_err(|error| error.to_string()),
        Source::Technic => TECHNIC.download(version, into, None).await.map_err(|error| error.to_string()),
    }
}

/// The newest version that fits the Minecraft version the filters ask for, or the newest of all.
/// Platforms list their versions newest first.
fn pick_version(versions: Vec<ContentVersion>, game_version: Option<&str>) -> Option<ContentVersion> {
    match game_version {
        Some(wanted) => versions
            .into_iter()
            .find(|version| version.minecraft_versions.iter().any(|known| known == wanted)),
        None => versions.into_iter().next(),
    }
}

/// The folder one download is written into, named for its platform, project and version.
fn download_folder(project: &Project, version: &ContentVersion) -> String {
    shared_library::download_folder(project.source.key(), &project.project_id, &version.id)
}

/// Downloads a project's newest matching version into its shared folder.
async fn download_project(project: Project, shared: PathBuf) -> Result<SharedRecord, String> {
    let versions = versions_of(project.source, project.content, &project.project_id).await?;
    let version = pick_version(versions, project.game_version.as_deref()).ok_or_else(|| lang::t("discover.no_version").to_string())?;
    let folder = download_folder(&project, &version);
    download_version(project.source, &version, &shared.join(&folder)).await?;
    let sizes: Vec<u64> = version.files.iter().filter_map(|file| file.size).collect();
    Ok(SharedRecord {
        kind: shared_library::folder_name(project.content).to_string(),
        platform: project.source.key().to_string(),
        project_id: project.project_id,
        version_id: version.id,
        name: project.name,
        path: folder,
        size: (!sizes.is_empty()).then(|| sizes.iter().sum()),
    })
}

/// Reads a modpack, downloading its archive into the shared folder when it has one, and returns the install that makes its instance.
async fn prepare_modpack(project: Project, paths: LauncherPaths, taken: Vec<String>) -> Result<InstallRequest, String> {
    let versions = versions_of(project.source, project.content, &project.project_id).await?;
    let version = pick_version(versions, project.game_version.as_deref()).ok_or_else(|| lang::t("discover.no_version").to_string())?;
    let folder = download_folder(&project, &version);
    let shared = shared_library::folder(&paths, ContentType::Modpack).join(&folder);
    let (modpack, archive) = match project.source {
        Source::Modrinth => {
            let archive = pack_archive(download_version(project.source, &version, &shared).await?)?;
            let modpack = MODRINTH.parse_modpack(&archive).map_err(|error| error.to_string())?;
            (modpack, Some(archive))
        }
        Source::CurseForge => {
            let archive = pack_archive(download_version(project.source, &version, &shared).await?)?;
            let modpack = curseforge()?.parse_modpack(&archive).map_err(|error| error.to_string())?;
            (modpack, Some(archive))
        }
        Source::Ftb => {
            let modpack = FTB.modpack(&version).await.map_err(|error| error.to_string())?;
            write_manifest(&shared, &modpack)?;
            (modpack, None)
        }
        Source::AtLauncher => {
            let modpack = ATLAUNCHER.modpack(&version).await.map_err(|error| error.to_string())?;
            (modpack, None)
        }
        // A Solder build names its mods on its server, so nothing is downloaded before the install.
        Source::Technic if version.files.is_empty() => {
            let modpack = TECHNIC.modpack(&version).await.map_err(|error| error.to_string())?;
            (modpack, None)
        }
        // A zip pack names its zip as its only file. The zip is read for its loader, then unpacked. Its file
        // name is the version's, and it names no Minecraft version unless it is a Forge pack, so the
        // project's name and the version's Minecraft version stand in for them.
        Source::Technic => {
            let archive = pack_archive(download_version(project.source, &version, &shared).await?)?;
            let mut modpack = TECHNIC.parse_modpack(&archive).map_err(|error| error.to_string())?;
            modpack.name = project.name.clone();
            if modpack.minecraft.is_empty() {
                modpack.minecraft = version.minecraft_versions.first().cloned().unwrap_or_default();
            }
            (modpack, Some(archive))
        }
    };
    let mut payload = Payload::from_modpack(&modpack, archive);
    if project.source == Source::CurseForge {
        payload.files = resolve_curseforge_files(payload.files).await?;
    }
    let (biome, seed) = discover::scene_for(project.source, &project.project_id);
    // The pack's own picture becomes the instance's icon and banner. Without one, the rose symbol and the
    // scene stand in.
    let art = match project.icon_url.clone() {
        Some(url) => discover::pack_art(url).await,
        None => None,
    };
    let (icon, banner) = match art {
        Some(art) => (IconSource::Uploaded(art.icon), Some(art.banner)),
        None => (
            IconSource::Composed {
                background: "rose".to_string(),
                symbol: "grass".to_string(),
            },
            None,
        ),
    };
    let directory_name = naming::unique_directory_name(&paths.instances(), &naming::clean_directory_name(&modpack.name), &taken);
    Ok(InstallRequest {
        paths,
        name: modpack.name,
        directory_name,
        mc_version: modpack.minecraft,
        loader: loader_of(modpack.loader),
        loader_version: modpack.loader_version,
        java_component: None,
        icon,
        biome,
        art_seed: seed,
        banner,
        id: install::new_id(),
        source_url: project.web_url,
        source_platform: Some(project.source.key().to_string()),
        source_project_id: Some(project.project_id),
        modpack: Some(payload),
    })
}

/// The archive a modpack downloaded as: its first `.mrpack` or `.zip`, or the first file.
fn pack_archive(files: Vec<PathBuf>) -> Result<PathBuf, String> {
    let is_archive = |path: &PathBuf| matches!(path.extension().and_then(|ext| ext.to_str()), Some("mrpack" | "zip"));
    files
        .iter()
        .find(|path| is_archive(path))
        .or_else(|| files.first())
        .cloned()
        .ok_or_else(|| lang::t("discover.no_version").to_string())
}

/// The loader a modpack names, as an instance's loader. A pack with no loader is vanilla.
fn loader_of(name: Option<&str>) -> Loader {
    match name {
        Some("fabric") => Loader::Fabric,
        Some("forge") => Loader::Forge,
        Some("neoforge") => Loader::NeoForge,
        Some("quilt") => Loader::Quilt,
        _ => Loader::Vanilla,
    }
}

/// Keeps an FTB modpack's file list in its shared folder, since FTB packs are not one archive.
fn write_manifest(dir: &Path, modpack: &Modpack) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let files: Vec<serde_json::Value> = modpack
        .files
        .iter()
        .map(|file| serde_json::json!({ "path": file.path, "url": file.url, "sha1": file.sha1, "size": file.size }))
        .collect();
    let manifest = serde_json::json!({
        "name": modpack.name,
        "minecraft": modpack.minecraft,
        "loader": modpack.loader,
        "loader_version": modpack.loader_version,
        "files": files,
    });
    let bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?;
    std::fs::write(dir.join("modpack.json"), bytes).map_err(|error| error.to_string())
}

/// Resolves the CurseForge files of a modpack to their download urls, a few at a time.
async fn resolve_curseforge_files(files: Vec<ModpackFile>) -> Result<Vec<ModpackFile>, String> {
    let mut resolved = Vec::with_capacity(files.len());
    let mut remaining = files.into_iter();
    loop {
        let batch: Vec<ModpackFile> = remaining.by_ref().take(RESOLVE_AT_ONCE).collect();
        if batch.is_empty() {
            break;
        }
        let mut calls = tokio::task::JoinSet::new();
        for file in batch {
            calls.spawn(resolve_one(file));
        }
        while let Some(joined) = calls.join_next().await {
            resolved.push(joined.map_err(|error| error.to_string())??);
        }
    }
    Ok(resolved)
}

/// The file with its download url filled in, asked of CurseForge when the modpack named only ids.
async fn resolve_one(mut file: ModpackFile) -> Result<ModpackFile, String> {
    if !file.url.is_empty() {
        return Ok(file);
    }
    let (Some(project), Some(id)) = (file.project_id.as_deref(), file.file_id.as_deref()) else {
        return Err(lang::t_with("discover.install.no_url", &[("file", file.path.as_str())]).to_string());
    };
    file.url = curseforge()?.file_url(project, id).await.map_err(|error| error.to_string())?;
    Ok(file)
}

/// The task for one search: the answer comes back tagged with the search's generation and offset.
fn search_task(search: Search) -> Task<LodestoneAppMessage> {
    let Search { generation, key, offset } = search;
    let source = key.source;
    let query = key.query(offset);
    Task::perform(run_search(source, query), move |(result, elapsed_ms)| {
        LodestoneAppMessage::Discover(Message::SearchFinished {
            key,
            offset,
            generation,
            result,
            elapsed_ms,
        })
    })
}

/// The effects that need nothing from the launcher's state.
fn plain_effect(effect: Effect) -> Task<LodestoneAppMessage> {
    match effect {
        Effect::Debounce(generation) => Task::perform(
            async move {
                tokio::time::sleep(DEBOUNCE).await;
                generation
            },
            |generation| LodestoneAppMessage::Discover(Message::SearchDue(generation)),
        ),
        Effect::Search(search) => search_task(search),
        Effect::Facets(source, content) => Task::perform(load_facets(source, content), move |result| {
            LodestoneAppMessage::Discover(Message::FacetsLoaded { source, content, result })
        }),
        Effect::Open(url) => super::instances::open_source(url),
        Effect::Focus => operation::focus(Id::new(SEARCH_ID)),
        Effect::ScrollToTop => operation::scroll_to(Id::new(CONTENT_ID), AbsoluteOffset { x: None, y: Some(0.0) }),
        _ => Task::none(),
    }
}

impl LodestoneLauncher {
    /// Routes one message of the Discover page: the page updates, its effects run, and the icons its
    /// results need start loading. A changed platform is remembered for the next launch.
    pub(super) fn discover_message(&mut self, message: Message) -> Task<LodestoneAppMessage> {
        let before = self.discover.source();
        let effects = self.discover.update(message);
        let mut tasks: Vec<Task<LodestoneAppMessage>> = Vec::new();
        if self.discover.source() != before {
            tasks.push(self.remember_discover_source());
        }
        tasks.extend(effects.into_iter().map(|effect| self.discover_effect(effect)));
        tasks.push(self.icon_tasks());
        Task::batch(tasks)
    }

    /// Starts the page's first search when the page is opened, and loads the icons its results need.
    pub(super) fn open_discover(&mut self) -> Task<LodestoneAppMessage> {
        let effects = self.discover.opening();
        let mut tasks: Vec<Task<LodestoneAppMessage>> = effects.into_iter().map(|effect| self.discover_effect(effect)).collect();
        tasks.push(self.icon_tasks());
        Task::batch(tasks)
    }

    /// Carries out one effect of the page.
    fn discover_effect(&mut self, effect: Effect) -> Task<LodestoneAppMessage> {
        match effect {
            Effect::Download(project) => self.download_task(project),
            Effect::Install(project) => self.install_task(project),
            Effect::Queue(request) => self.queue_install(*request),
            Effect::Record(record) => self.record_download(record),
            Effect::Notify(error) => self.notify_discover(error),
            Effect::OpenInstance(id) => self.open_instance_page(&id, Tab::Overview),
            effect => plain_effect(effect),
        }
    }

    /// Downloads a project into its shared folder. The answer comes back as `Downloaded`.
    fn download_task(&self, project: Project) -> Task<LodestoneAppMessage> {
        let kind = shared_library::folder_name(project.content).to_string();
        let platform = project.source.key().to_string();
        let project_id = project.project_id.clone();
        let shared = shared_library::folder(&self.paths, project.content);
        Task::perform(download_project(project, shared), move |result| {
            LodestoneAppMessage::Discover(Message::Downloaded {
                kind,
                platform,
                project_id,
                result,
            })
        })
    }

    /// Downloads a modpack and prepares its install. The answer comes back as `Prepared`.
    fn install_task(&self, project: Project) -> Task<LodestoneAppMessage> {
        let platform = project.source.key().to_string();
        let project_id = project.project_id.clone();
        let taken = self
            .downloads
            .reserved_directories()
            .into_iter()
            .chain(self.library.directory_names())
            .collect();
        Task::perform(prepare_modpack(project, self.paths.clone(), taken), move |result| {
            LodestoneAppMessage::Discover(Message::Prepared {
                platform,
                project_id,
                result: result.map(Box::new),
            })
        })
    }

    /// Queues a prepared install with the downloads, which start it as a slot frees up.
    fn queue_install(&mut self, request: InstallRequest) -> Task<LodestoneAppMessage> {
        self.downloads.enqueue(request);
        self.downloads.set_open(true);
        self.sync_pending();
        Task::batch([self.start_ready_downloads(), self.refresh_totals()])
    }

    /// Saves a shared download, so the page still lists it after a restart.
    fn record_download(&self, record: SharedRecord) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        Task::future(async move {
            if let Err(error) = db.record_shared_content(&record).await {
                warn!("recording the shared download failed: {error}");
            }
        })
        .discard()
    }

    /// Shows a problem with a Discover action to the user.
    fn notify_discover(&mut self, error: String) -> Task<LodestoneAppMessage> {
        self.alerts.show(Alert::error(error)).discard()
    }

    /// Loads the downloads on disk, once the database is open.
    pub(super) fn load_shared(&self) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        Task::perform(async move { db.shared_content().await }, LodestoneAppMessage::SharedLoaded)
    }

    /// Loads the icons the page's hits need, each one once.
    fn icon_tasks(&mut self) -> Task<LodestoneAppMessage> {
        let urls = self.discover.wanted_icons();
        Task::batch(urls.into_iter().map(|url| {
            let answer = url.clone();
            Task::perform(discover::load_icon(url), move |picture| {
                LodestoneAppMessage::Discover(Message::IconReady { url: answer, picture })
            })
        }))
    }

    /// The installs the page shows: a pack whose instance exists is installed, and one still in the
    /// downloads queue is queued. The root sends this after every message, so the cards follow both.
    pub(super) fn sync_discover_installs(&mut self) {
        let mut installs = HashMap::new();
        for job in self.downloads.jobs() {
            if matches!(job.state, JobState::Done | JobState::Failed(_)) {
                continue;
            }
            if let (Some(platform), Some(project)) = (&job.request.source_platform, &job.request.source_project_id) {
                installs.insert((platform.clone(), project.clone()), InstallState::Queued);
            }
        }
        for (platform, project, instance) in self.library.installed_projects() {
            installs.insert((platform.to_string(), project.to_string()), InstallState::Installed(instance.to_string()));
        }
        self.discover.set_installs(installs);
    }

    /// Writes the page's platform to the preferences on the blocking pool, keeping the other choices.
    fn remember_discover_source(&self) -> Task<LodestoneAppMessage> {
        let key = self.discover.source().key().to_string();
        Task::future(async move {
            let saved = tokio::task::spawn_blocking(move || {
                let mut preferences = Preferences::load();
                preferences.discover_source = Some(key);
                preferences.store()
            })
            .await;
            match saved {
                Ok(Ok(())) => {}
                Ok(Err(error)) => warn!("saving the Discover platform failed: {error}"),
                Err(error) => warn!("saving the Discover platform failed: {error}"),
            }
        })
        .discard()
    }
}
