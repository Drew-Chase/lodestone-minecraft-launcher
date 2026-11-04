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
    AtLauncher, BundledProject, ContentDetails, ContentType, ContentVersion, Facets, Ftb, Modpack, ModpackFile, Modrinth, Platform, SearchPage,
    SearchQuery, Technic,
};
use tracing::warn;

use super::LodestoneAppMessage;
use super::LodestoneLauncher;
use super::components::alert::Alert;
use super::downloads::{JobState, ModJob};
use super::install::InstallRequest;
use super::install::mods::add_to_instance;
use super::install::pack::{self, Page};
use super::lang;
use super::mod_actions::replace_jar;
use super::pages::discover::{
    self, AddRequest, CONTENT_ID, DiscoverView, Effect, InstallState, InstanceChoice, Message, Project, SEARCH_ID, Search, Source,
};
use super::pages::instance::Tab;
use super::paths::LauncherPaths;
use super::preferences::Preferences;
use super::shared_library::{self, ModSource, SharedRecord};

/// How long the page waits after the last keystroke before it searches.
const DEBOUNCE: Duration = Duration::from_millis(300);
/// How long a picture may take to load before the page gives up on it.
const ICON_TIMEOUT: Duration = Duration::from_secs(30);
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

/// The layout the page opens on: the one picked last, when it names one. Cards in a grid otherwise.
pub(crate) fn opening_view() -> DiscoverView {
    Preferences::load()
        .discover_view
        .as_deref()
        .and_then(DiscoverView::from_key)
        .unwrap_or_default()
}

/// One client per platform, shared by every request, so connections are reused.
pub(crate) static MODRINTH: LazyLock<Platform<Modrinth>> = LazyLock::new(Platform::<Modrinth>::new);
static CURSEFORGE: LazyLock<Option<Platform<lodestone_platforms::CurseForge>>> =
    LazyLock::new(|| curseforge_key().map(Platform::<lodestone_platforms::CurseForge>::with_api_key));
static ATLAUNCHER: LazyLock<Platform<AtLauncher>> = LazyLock::new(Platform::<AtLauncher>::new);
static FTB: LazyLock<Platform<Ftb>> = LazyLock::new(Platform::<Ftb>::new);
static TECHNIC: LazyLock<Platform<Technic>> = LazyLock::new(Platform::<Technic>::new);

/// The CurseForge handle, or the message that says why there is none.
pub(crate) fn curseforge() -> Result<&'static Platform<lodestone_platforms::CurseForge>, String> {
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
pub(super) async fn versions_of(source: Source, content: ContentType, project_id: &str) -> Result<Vec<ContentVersion>, String> {
    match source {
        Source::Modrinth => MODRINTH.versions(project_id, content).await.map_err(|error| error.to_string()),
        Source::CurseForge => curseforge()?.versions(project_id, content).await.map_err(|error| error.to_string()),
        Source::AtLauncher => ATLAUNCHER.versions(project_id, content).await.map_err(|error| error.to_string()),
        Source::Ftb => FTB.versions(project_id, content).await.map_err(|error| error.to_string()),
        Source::Technic => TECHNIC.versions(project_id, content).await.map_err(|error| error.to_string()),
    }
}

/// The details of one project on `source`: its description, facts and gallery.
pub(super) async fn details_of(source: Source, content: ContentType, project_id: &str) -> Result<ContentDetails, String> {
    match source {
        Source::Modrinth => MODRINTH.details(project_id, content).await.map_err(|error| error.to_string()),
        Source::CurseForge => curseforge()?.details(project_id, content).await.map_err(|error| error.to_string()),
        Source::AtLauncher => ATLAUNCHER.details(project_id, content).await.map_err(|error| error.to_string()),
        Source::Ftb => FTB.details(project_id, content).await.map_err(|error| error.to_string()),
        Source::Technic => TECHNIC.details(project_id, content).await.map_err(|error| error.to_string()),
    }
}

/// The projects a Modrinth modpack release bundles, for its contents list.
pub(super) async fn bundled_of(version: &ContentVersion) -> Result<Vec<BundledProject>, String> {
    MODRINTH.bundled(version).await.map_err(|error| error.to_string())
}

/// The release a project's action takes: the one the user picked, or else the newest that fits the filters.
fn chosen_release(versions: Vec<ContentVersion>, project: &Project) -> Option<ContentVersion> {
    match project.version_id.as_deref() {
        Some(id) => versions.into_iter().find(|version| version.id == id),
        None => pick_version(versions, project.game_version.as_deref()),
    }
}

/// Downloads one version's files into `into`, returning their paths.
pub(super) async fn download_version(source: Source, version: &ContentVersion, into: &Path) -> Result<Vec<PathBuf>, String> {
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
    let version = chosen_release(versions, &project).ok_or_else(|| lang::t("discover.no_version").to_string())?;
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

/// Reads a modpack, downloading its archive into the shared folder when it has one, and returns the install that makes its instance,
/// with the archive's size (zero when it has no archive).
pub(crate) async fn prepare_modpack(project: Project, paths: LauncherPaths, taken: Vec<String>) -> Result<(InstallRequest, u64), String> {
    let versions = versions_of(project.source, project.content, &project.project_id).await?;
    let version = chosen_release(versions, &project).ok_or_else(|| lang::t("discover.no_version").to_string())?;
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
    // The archive's size is what an import from the page lists, so it is read before the archive is taken.
    let bytes = archive
        .as_deref()
        .and_then(|path| std::fs::metadata(path).ok())
        .map_or(0, |meta| meta.len());
    let source = Some(project.source);
    let page = Page {
        source: project.source,
        project_id: project.project_id,
        url: project.web_url,
        icon_url: project.icon_url,
    };
    let request = pack::request(&paths, &taken, modpack, archive, source, Some(page), Some(version.version_number.clone())).await?;
    Ok((request, bytes))
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
pub(crate) async fn resolve_curseforge_files(files: Vec<ModpackFile>) -> Result<Vec<ModpackFile>, String> {
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

/// The file with its download url, its name in `mods/` and its hash filled in, asked of CurseForge when the modpack
/// named only ids. The manifest gives no path, so the jar's own name places it.
async fn resolve_one(mut file: ModpackFile) -> Result<ModpackFile, String> {
    if !file.url.is_empty() {
        return Ok(file);
    }
    let (Some(project), Some(id)) = (file.project_id.as_deref(), file.file_id.as_deref()) else {
        return Err(lang::t_with("discover.install.no_url", &[("file", file.path.as_str())]).to_string());
    };
    let jar = curseforge()?.file(project, id).await.map_err(|error| error.to_string())?;
    file.path = format!("mods/{}", jar.name);
    file.url = jar.url;
    file.sha1 = jar.sha1;
    file.size = jar.size;
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

/// The answer to a mod's add or replacement, as the Discover page receives it.
fn mod_answer(job: u64, instance: String, platform: String, project_id: String, result: Result<Vec<ModSource>, String>) -> LodestoneAppMessage {
    LodestoneAppMessage::Discover(Message::ModAdded {
        job,
        instance,
        platform,
        project_id,
        result,
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
        self.sync_discover_instances();
        if let Message::ModAdded { job, instance, result, .. } = &message {
            self.mod_job_ended(*job, instance, result);
        }
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
        self.sync_discover_instances();
        let effects = self.discover.opening();
        let mut tasks: Vec<Task<LodestoneAppMessage>> = effects.into_iter().map(|effect| self.discover_effect(effect)).collect();
        tasks.push(self.icon_tasks());
        Task::batch(tasks)
    }

    /// Carries out one effect of the page.
    pub(super) fn discover_effect(&mut self, effect: Effect) -> Task<LodestoneAppMessage> {
        match effect {
            Effect::Download(project) => self.download_task(project),
            Effect::Install(project) => self.install_task(project),
            Effect::Queue(request) => self.queue_install(*request),
            Effect::Record(record) => self.record_download(record),
            Effect::Notify(error) => self.notify_discover(error),
            Effect::OpenInstance(id) => self.open_instance_page(&id, Tab::Mods),
            Effect::RememberView(view) => self.remember_discover_view(view),
            Effect::Back => self.back_from_discover(),
            Effect::OpenResource(listing) => self.open_resource(listing),
            Effect::AddMod(request) => self.add_mod_task(*request),
            Effect::ReplaceMod { request, old_file } => self.replace_mod_task(*request, old_file),
            Effect::RecordModSources(records) => self.record_mod_sources(records),
            effect => plain_effect(effect),
        }
    }

    /// Adds a mod to its instance's `mods/` folder, with the required dependencies the folder lacks. The add is a
    /// downloads job, and its answer comes back as `ModAdded`.
    fn add_mod_task(&mut self, request: AddRequest) -> Task<LodestoneAppMessage> {
        let job = self.enqueue_mod_job(&request, false);
        let instance = request.instance.id.clone();
        let platform = request.project.source.key().to_string();
        let project_id = request.project.project_id.clone();
        let Some(directory) = self.library.instance(&instance).map(|found| found.directory_name) else {
            return Task::done(mod_answer(
                job,
                instance,
                platform,
                project_id,
                Err(lang::t("discover.mods.gone").to_string()),
            ));
        };
        let mods = self.paths.instance(&directory).join("mods");
        Task::perform(add_to_instance(request, mods), move |result| {
            mod_answer(job, instance, platform, project_id, result)
        })
    }

    /// Swaps the jar `old_file` of a mod the instance has for the release the request names. The swap is a downloads
    /// job, and its answer comes back as `ModAdded`, as an add's does.
    fn replace_mod_task(&mut self, request: AddRequest, old_file: String) -> Task<LodestoneAppMessage> {
        let job = self.enqueue_mod_job(&request, true);
        let instance = request.instance.id.clone();
        let platform = request.project.source.key().to_string();
        let project_id = request.project.project_id.clone();
        let Some(directory) = self.library.instance(&instance).map(|found| found.directory_name) else {
            return Task::done(mod_answer(
                job,
                instance,
                platform,
                project_id,
                Err(lang::t("discover.mods.gone").to_string()),
            ));
        };
        let Some(db) = self.db.clone() else {
            let error = lang::t("new_instance.database_missing").to_string();
            return Task::done(mod_answer(job, instance, platform, project_id, Err(error)));
        };
        let mods = self.paths.instance(&directory).join("mods");
        let keep_disabled = old_file.ends_with(".disabled");
        Task::perform(replace_jar(db, request, mods, old_file, keep_disabled), move |result| {
            mod_answer(job, instance, platform, project_id, result)
        })
    }

    /// Starts the downloads job that lists a mod's add or replacement until the answer ends it.
    fn enqueue_mod_job(&mut self, request: &AddRequest, replacing: bool) -> u64 {
        self.downloads.enqueue_mod(ModJob {
            name: request.project.name.clone(),
            source: request.project.source,
            instance: request.instance.name.clone(),
            replacing,
        })
    }

    /// Ends the downloads job of a mod's add or replacement, and shows the mods it landed on the Mods tab when that
    /// instance is on screen. The page is told the answer afterwards.
    fn mod_job_ended(&mut self, job: u64, instance: &str, result: &Result<Vec<ModSource>, String>) {
        self.downloads.finish(job, result.as_ref().map(|_| ()).map_err(Clone::clone));
        if let Ok(records) = result
            && self.instance_page.instance().is_some_and(|shown| shown.id == instance)
        {
            for record in records {
                self.instance_page.mods_mut().apply_source(record.clone());
            }
        }
    }

    /// Saves the mods a platform install added to an instance, so the Mods tab still shows where they came from after
    /// a restart. The instances reload too, so the mod count on their cards is current.
    fn record_mod_sources(&mut self, records: Vec<ModSource>) -> Task<LodestoneAppMessage> {
        let reload = self.load_instances();
        let Some(db) = self.db.clone() else {
            return reload;
        };
        let save = Task::future(async move {
            if let Err(error) = db.record_mod_sources(&records).await {
                warn!("recording the instance's mods failed: {error}");
            }
        })
        .discard();
        Task::batch([save, reload])
    }

    /// Loads the mods that came from a platform, once the database is open.
    pub(super) fn load_mod_sources(&self) -> Task<LodestoneAppMessage> {
        let Some(db) = self.db.clone() else {
            return Task::none();
        };
        Task::perform(async move { db.mod_sources().await }, LodestoneAppMessage::ModSourcesLoaded)
    }

    /// Hands the Discover page the instances mods can be added to, as the library holds them now.
    pub(super) fn sync_discover_instances(&mut self) {
        let choices: Vec<InstanceChoice> = self.library.instances().iter().map(InstanceChoice::of).collect();
        self.discover.set_instances(choices);
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
        let paths = self.paths.clone();
        Task::perform(
            async move { prepare_modpack(project, paths, taken).await.map(|(request, _)| request) },
            move |result| {
                LodestoneAppMessage::Discover(Message::Prepared {
                    platform,
                    project_id,
                    result: result.map(Box::new),
                })
            },
        )
    }

    /// Queues a prepared install with the downloads, which start it as a slot frees up.
    pub(crate) fn queue_install(&mut self, request: InstallRequest) -> Task<LodestoneAppMessage> {
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
            // A picture that takes too long is given up, so its place in the loads is freed for the rest.
            let load = async move { tokio::time::timeout(ICON_TIMEOUT, discover::load_icon(url)).await.ok().flatten() };
            Task::perform(load, move |picture| {
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
            let Some(request) = job.install() else {
                continue;
            };
            if let (Some(platform), Some(project)) = (&request.source_platform, &request.source_project_id) {
                installs.insert((platform.clone(), project.clone()), InstallState::Queued);
            }
        }
        for (platform, project, instance) in self.library.installed_projects() {
            installs.insert((platform.to_string(), project.to_string()), InstallState::Installed(instance.to_string()));
        }
        self.discover.set_installs(installs);
    }

    /// Writes the page's layout to the preferences on the blocking pool, keeping the other choices, so it opens on it next time.
    fn remember_discover_view(&self, view: DiscoverView) -> Task<LodestoneAppMessage> {
        let key = view.key().to_string();
        Task::future(async move {
            let saved = tokio::task::spawn_blocking(move || {
                let mut preferences = Preferences::load();
                preferences.discover_view = Some(key);
                preferences.store()
            })
            .await;
            match saved {
                Ok(Ok(())) => {}
                Ok(Err(error)) => warn!("saving the Discover layout failed: {error}"),
                Err(error) => warn!("saving the Discover layout failed: {error}"),
            }
        })
        .discard()
    }

    /// Returns from the Discover page to the instance it was opened from. The instance's page is still as it was left, so
    /// it is shown again and its mods are read again, since the page may have added some.
    fn back_from_discover(&mut self) -> Task<LodestoneAppMessage> {
        self.discover.set_origin(None);
        let back = self.navigate(super::pages::Page::Instance);
        Task::batch([back, self.rescan_mods_on_return()])
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
