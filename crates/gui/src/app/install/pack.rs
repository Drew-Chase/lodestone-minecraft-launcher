//! A modpack's install: the files the pack places and the archives it unpacks, the records of the mods it names, and the
//! instance's look. Discover and the Import dialog both turn a parsed modpack into its install here, once they have the
//! pack's data and its archive on disk.

use std::path::PathBuf;

use lodestone_platforms::Modpack;

use super::modpack::Payload;
use super::package::random_scene;
use super::{IconSource, InstallRequest, new_id};
use crate::app::discover::resolve_curseforge_files;
use crate::app::mod_sources::{curseforge_sources, modrinth_sources};
use crate::app::pages::discover::{self, Source};
use crate::app::pages::library::instance::Loader;
use crate::app::pages::library::new_instance::naming;
use crate::app::paths::LauncherPaths;

/// The platform page a modpack was taken from: its project there, and the project's page and picture when it has them.
/// The instance keeps the page and the project, so its own page can open the pack again.
pub(crate) struct Page {
    /// The platform the pack comes from.
    pub source: Source,
    /// The project's id on the platform.
    pub project_id: String,
    /// The project's page on the platform, when it has one.
    pub url: Option<String>,
    /// The project's picture on the platform. It becomes the instance's icon and banner, when it can be read.
    pub icon_url: Option<String>,
}

/// The install of `modpack`. `archive` is the pack's own file on disk, which its overrides come from. `source` is the
/// platform the pack is read as, which decides how its files are resolved and recorded. `page` is where the pack was
/// taken from, when it has a page. The folder is named after the pack and kept clear of `taken`.
pub(crate) async fn request(
    paths: &LauncherPaths,
    taken: &[String],
    modpack: Modpack,
    archive: Option<PathBuf>,
    source: Option<Source>,
    page: Option<Page>,
) -> Result<InstallRequest, String> {
    let mut payload = Payload::from_modpack(&modpack, archive);
    match source {
        Some(Source::CurseForge) => {
            payload.files = resolve_curseforge_files(payload.files).await?;
            payload.sources = curseforge_sources(&payload.files);
        }
        Some(Source::Modrinth) => payload.sources = modrinth_sources(&payload.files).await,
        _ => {}
    }
    let (biome, art_seed) = match &page {
        Some(page) => discover::scene_for(page.source, &page.project_id),
        None => random_scene(),
    };
    // The pack's own picture becomes the instance's icon and banner. Without one, the rose symbol and the scene stand in.
    let art = match page.as_ref().and_then(|page| page.icon_url.clone()) {
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
    let directory_name = naming::unique_directory_name(&paths.instances(), &naming::clean_directory_name(&modpack.name), taken);
    Ok(InstallRequest {
        paths: paths.clone(),
        name: modpack.name,
        directory_name,
        mc_version: modpack.minecraft,
        loader: loader_of(modpack.loader),
        loader_version: modpack.loader_version,
        java_component: None,
        icon,
        biome,
        art_seed,
        banner,
        id: new_id(),
        source_url: page.as_ref().and_then(|page| page.url.clone()),
        source_platform: page.as_ref().map(|page| page.source.key().to_string()),
        source_project_id: page.map(|page| page.project_id),
        modpack: Some(payload),
        settings: Default::default(),
    })
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

#[cfg(test)]
#[path = "../../../tests/app/install/pack.rs"]
mod tests;
