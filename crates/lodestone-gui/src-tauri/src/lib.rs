use serde_json::Value;

use hopper_mc::{
    ContentType, ModrinthProvider, Platform, Sort,
    ModProvider, PackProvider, DatapackProvider, ResourcePackProvider,
    ShaderPackProvider, WorldProvider,
};

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
) -> Result<Vec<Value>, String> {
    let s = parse_sort(&sort);
    let ct = parse_content_type(&content_type);
    let q = query.as_deref();

    match platform.as_str() {
        "modrinth" => search_on_platform(q, s, Platform::Modrinth, ct, page, per_page).await,
        "curseforge" => search_on_platform(q, s, Platform::CurseForge, ct, page, per_page).await,
        _ => {
            // "all" — query both in parallel, merge results
            let (mr_res, cf_res): (Result<Vec<Value>, String>, Result<Vec<Value>, String>) =
                tokio::join!(
                    search_on_platform(q, s, Platform::Modrinth, ct, page, per_page),
                    search_on_platform(q, s, Platform::CurseForge, ct, page, per_page),
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
    page: u32,
    per_page: u32,
) -> Result<Vec<Value>, String> {
    let mr = ModrinthProvider::shared();

    macro_rules! search {
        ($find_fn:ident) => {{
            match platform {
                Platform::Modrinth => mr
                    .$find_fn(query, sort, page, per_page)
                    .await
                    .map(|v| v.into_iter().filter_map(|i| serde_json::to_value(i).ok()).collect())
                    .map_err(|e| e.to_string()),
                // CurseForge requires an API key; return empty until a
                // keyed provider is wired (via the embedded-key feature in
                // production builds).
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![search_content, get_content])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
