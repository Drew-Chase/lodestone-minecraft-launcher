use serde::{Deserialize, Serialize};
use serde_json::Value;

use hopper_mc::{
    ContentType, ModrinthProvider, Platform, SearchFilters, Sort,
    ModProvider, PackProvider, DatapackProvider, ResourcePackProvider,
    ShaderPackProvider, WorldProvider,
};

/// Filter parameters received from the frontend.
#[derive(Debug, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct FilterParams {
    categories: Vec<String>,
    loaders: Vec<String>,
    versions: Vec<String>,
    environment: Vec<String>,
}

impl FilterParams {
    fn to_search_filters(&self) -> SearchFilters {
        let mut sf = SearchFilters {
            categories: self.categories.clone(),
            loaders: self.loaders.clone(),
            versions: self.versions.clone(),
            ..Default::default()
        };
        // Map environment strings to client_side / server_side facets.
        for env in &self.environment {
            match env.to_ascii_lowercase().as_str() {
                "client" => sf.client_side = Some("required".into()),
                "server" => sf.server_side = Some("required".into()),
                "client & server" => {
                    sf.client_side = Some("required".into());
                    sf.server_side = Some("required".into());
                }
                _ => {}
            }
        }
        sf
    }
}
#[cfg(debug_assertions)]
use hopper_mc::CurseForgeProvider;

#[cfg(debug_assertions)]
fn curseforge_provider() -> &'static CurseForgeProvider {
    use std::sync::OnceLock;
    static CF: OnceLock<CurseForgeProvider> = OnceLock::new();
    CF.get_or_init(|| {
        let key = std::env::var("CURSEFORGE_API_KEY").ok();
        CurseForgeProvider::new_with_key(key)
    })
}

fn parse_sort(s: &str) -> Sort {
    match s {
        "downloads" => Sort::Downloads,
        "follows" => Sort::Follows,
        "latest" => Sort::Latest,
        "updated" => Sort::Updated,
        _ => Sort::Relevance,
    }
}

fn parse_content_type(s: &str) -> ContentType {
    match s {
        "modpack" => ContentType::Modpack,
        "datapack" => ContentType::Datapack,
        "resourcepack" => ContentType::ResourcePack,
        "shaderpack" => ContentType::ShaderPack,
        "world" => ContentType::World,
        _ => ContentType::Mod,
    }
}

/// Search for content across one or both platforms. When `platform` is
/// `"all"`, Modrinth and CurseForge are queried in parallel and the
/// results merged.
#[tauri::command]
async fn search_content(
    query: Option<String>,
    sort: String,
    platform: String,
    content_type: String,
    page: u32,
    per_page: u32,
    filters: Option<FilterParams>,
) -> Result<Vec<Value>, String> {
    let s = parse_sort(&sort);
    let ct = parse_content_type(&content_type);
    let q = query.as_deref();
    let sf = filters.unwrap_or_default().to_search_filters();

    match platform.as_str() {
        "modrinth" => search_on_platform(q, s, Platform::Modrinth, ct, &sf, page, per_page).await,
        "curseforge" => search_on_platform(q, s, Platform::CurseForge, ct, &sf, page, per_page).await,
        _ => {
            // "all" — query both in parallel, merge results
            let (mr_res, cf_res): (Result<Vec<Value>, String>, Result<Vec<Value>, String>) =
                tokio::join!(
                    search_on_platform(q, s, Platform::Modrinth, ct, &sf, page, per_page),
                    search_on_platform(q, s, Platform::CurseForge, ct, &sf, page, per_page),
                );
            let mut combined = mr_res.unwrap_or_default();
            combined.extend(cf_res.unwrap_or_default());
            Ok(combined)
        }
    }
}

async fn search_on_platform(
    query: Option<&str>,
    sort: Sort,
    platform: Platform,
    ct: ContentType,
    filters: &SearchFilters,
    page: u32,
    per_page: u32,
) -> Result<Vec<Value>, String> {
    let mr = ModrinthProvider::shared();

    macro_rules! search {
        ($find_fn:ident) => {{
            match platform {
                Platform::Modrinth => mr
                    .$find_fn(query, sort, filters, page, per_page)
                    .await
                    .map(|v| v.into_iter().filter_map(|i| serde_json::to_value(i).ok()).collect())
                    .map_err(|e| e.to_string()),
                #[cfg(debug_assertions)]
                Platform::CurseForge => {
                    let cf = curseforge_provider();
                    cf.$find_fn(query, sort, filters, page, per_page)
                        .await
                        .map(|v| v.into_iter().filter_map(|i| serde_json::to_value(i).ok()).collect())
                        .map_err(|e| e.to_string())
                }
                #[cfg(not(debug_assertions))]
                Platform::CurseForge => Ok(Vec::new()),
                _ => Err(format!("{platform:?} is not yet implemented")),
            }
        }};
    }

    match ct {
        ContentType::Mod => search!(find_mods),
        ContentType::Modpack => search!(find_packs),
        ContentType::Datapack => search!(find_datapacks),
        ContentType::ResourcePack => search!(find_resourcepacks),
        ContentType::ShaderPack => search!(find_shaderpacks),
        ContentType::World => search!(find_worlds),
    }
}

/// Fetch a single content item by id/slug from a specific platform.
#[tauri::command]
async fn get_content(
    id: String,
    platform: String,
    content_type: String,
) -> Result<Option<Value>, String> {
    let ct = parse_content_type(&content_type);
    let plat = match platform.as_str() {
        "curseforge" => Platform::CurseForge,
        _ => Platform::Modrinth,
    };
    let mr = ModrinthProvider::shared();

    macro_rules! get_item {
        ($get_fn:ident) => {{
            match plat {
                Platform::Modrinth => mr
                    .$get_fn(&id)
                    .await
                    .map(|opt| opt.and_then(|i| serde_json::to_value(i).ok()))
                    .map_err(|e| e.to_string()),
                #[cfg(debug_assertions)]
                Platform::CurseForge => {
                    let cf = curseforge_provider();
                    cf.$get_fn(&id)
                        .await
                        .map(|opt| opt.and_then(|i| serde_json::to_value(i).ok()))
                        .map_err(|e| e.to_string())
                }
                #[cfg(not(debug_assertions))]
                Platform::CurseForge => Ok(None),
                _ => Err(format!("{plat:?} is not yet implemented")),
            }
        }};
    }

    match ct {
        ContentType::Mod => get_item!(get_mod),
        ContentType::Modpack => get_item!(get_pack),
        ContentType::Datapack => get_item!(get_datapack),
        ContentType::ResourcePack => get_item!(get_resourcepack),
        ContentType::ShaderPack => get_item!(get_shaderpack),
        ContentType::World => get_item!(get_world),
    }
}

/// A Minecraft version entry returned to the frontend.
#[derive(Debug, Clone, Serialize)]
struct McVersion {
    id: String,
    version_type: String,
}

#[tauri::command]
async fn get_minecraft_versions() -> Result<Vec<McVersion>, String> {
    use tokio::sync::OnceCell;

    static VERSIONS: OnceCell<Vec<McVersion>> = OnceCell::const_new();

    let versions = VERSIONS
        .get_or_try_init(|| async {
            let manifest = piston_mc::manifest_v2::ManifestV2::fetch()
                .await
                .map_err(|e| e.to_string())?;
            Ok::<Vec<McVersion>, String>(
                manifest
                    .versions
                    .into_iter()
                    .map(|v| McVersion {
                        id: v.id,
                        version_type: v.release_type.to_string(),
                    })
                    .collect(),
            )
        })
        .await?;

    Ok(versions.clone())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            search_content,
            get_content,
            get_minecraft_versions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
