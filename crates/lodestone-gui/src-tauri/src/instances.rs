use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::Manager;
use tokio::sync::Mutex;

use lodestone_core::instance::{CreateInstanceParams, InstanceConfig, LoaderType};
use lodestone_core::instance_manager::InstanceManager;

use minecraft_modloaders::fabric::FabricVersions;
use minecraft_modloaders::forge::ForgeVersions;
use minecraft_modloaders::neoforge::NeoForgeVersions;
use minecraft_modloaders::quilt::QuiltVersions;

/// Shared instance manager state, initialized lazily on first use.
pub type InstanceManagerState = Arc<Mutex<Option<InstanceManager>>>;

pub async fn ensure_manager(
    state: &InstanceManagerState,
    app: &tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = state.lock().await;
    if guard.is_none() {
        let data_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
        let instances_dir = data_dir.join("instances");
        let manager = InstanceManager::new(&data_dir, instances_dir)
            .await
            .map_err(|e| format!("failed to initialize instance manager: {e}"))?;
        *guard = Some(manager);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Instance CRUD commands
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInstanceRequest {
    pub name: String,
    pub minecraft_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub java_version: Option<String>,
}

#[tauri::command]
pub async fn create_instance(
    request: CreateInstanceRequest,
    state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<InstanceConfig, String> {
    let loader = LoaderType::parse(&request.loader)
        .ok_or_else(|| format!("unknown loader: {}", request.loader))?;

    let params = CreateInstanceParams {
        name: request.name,
        minecraft_version: request.minecraft_version,
        loader,
        loader_version: request.loader_version,
        java_version: request.java_version,
    };

    ensure_manager(&state, &app).await?;
    let guard = state.lock().await;
    let mgr = guard.as_ref().unwrap();
    mgr.create(params)
        .await
        .map_err(|e| format!("failed to create instance: {e}"))
}

#[tauri::command]
pub async fn list_instances(
    state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<Vec<InstanceConfig>, String> {
    ensure_manager(&state, &app).await?;
    let guard = state.lock().await;
    let mgr = guard.as_ref().unwrap();
    mgr.list()
        .await
        .map_err(|e| format!("failed to list instances: {e}"))
}

#[tauri::command]
pub async fn delete_instance(
    id: i64,
    state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    ensure_manager(&state, &app).await?;
    let guard = state.lock().await;
    let mgr = guard.as_ref().unwrap();
    mgr.delete(id)
        .await
        .map_err(|e| format!("failed to delete instance: {e}"))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstanceRequest {
    pub id: i64,
    pub minecraft_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub java_version: Option<String>,
}

#[tauri::command]
pub async fn update_instance(
    request: UpdateInstanceRequest,
    state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let loader = LoaderType::parse(&request.loader)
        .ok_or_else(|| format!("unknown loader: {}", request.loader))?;
    ensure_manager(&state, &app).await?;
    let guard = state.lock().await;
    let mgr = guard.as_ref().unwrap();
    mgr.update(
        request.id,
        &request.minecraft_version,
        &loader,
        request.loader_version.as_deref(),
        request.java_version.as_deref(),
    )
    .await
    .map_err(|e| format!("failed to update instance: {e}"))
}

// ---------------------------------------------------------------------------
// Loader version fetching
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoaderVersionsResponse {
    pub versions: Vec<String>,
    pub recommended: Option<String>,
}

#[tauri::command]
pub async fn get_loader_versions(
    loader: String,
    minecraft_version: String,
) -> Result<LoaderVersionsResponse, String> {
    match loader.as_str() {
        "vanilla" => Ok(LoaderVersionsResponse {
            versions: vec![],
            recommended: None,
        }),
        "fabric" => {
            let versions = FabricVersions::fetch()
                .await
                .map_err(|e| format!("failed to fetch Fabric versions: {e}"))?;
            let loader_versions: Vec<String> =
                versions.loader.iter().map(|v| v.version.clone()).collect();
            let recommended = versions.get_latest_loader().map(|v| v.version.clone());
            Ok(LoaderVersionsResponse {
                versions: loader_versions,
                recommended,
            })
        }
        "forge" => {
            let versions = ForgeVersions::fetch()
                .await
                .map_err(|e| format!("failed to fetch Forge versions: {e}"))?;
            let forge_versions: Vec<String> = versions
                .get_versions(&minecraft_version)
                .unwrap_or_default()
                .into_iter()
                .map(|v| {
                    v.strip_prefix(&format!("{minecraft_version}-"))
                        .unwrap_or(v)
                        .to_string()
                })
                .collect();
            let recommended = forge_versions.first().cloned();
            Ok(LoaderVersionsResponse {
                versions: forge_versions,
                recommended,
            })
        }
        "neoforge" => {
            let versions = NeoForgeVersions::fetch()
                .await
                .map_err(|e| format!("failed to fetch NeoForge versions: {e}"))?;
            let nf_versions: Vec<String> = versions
                .get_versions(&minecraft_version)
                .into_iter()
                .rev()
                .map(|v| v.to_string())
                .collect();
            let recommended = nf_versions.first().cloned();
            Ok(LoaderVersionsResponse {
                versions: nf_versions,
                recommended,
            })
        }
        "quilt" => {
            let versions = QuiltVersions::fetch()
                .await
                .map_err(|e| format!("failed to fetch Quilt versions: {e}"))?;
            let loader_versions: Vec<String> =
                versions.loader.iter().map(|v| v.version.clone()).collect();
            let recommended = versions.get_latest_loader().map(|v| v.version.clone());
            Ok(LoaderVersionsResponse {
                versions: loader_versions,
                recommended,
            })
        }
        _ => Err(format!("unknown loader: {loader}")),
    }
}

// ---------------------------------------------------------------------------
// Java version detection
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaInfo {
    pub major_version: u8,
    pub label: String,
}

#[tauri::command]
pub async fn get_java_for_version(minecraft_version: String) -> Result<JavaInfo, String> {
    let manifest = piston_mc::manifest_v2::ManifestV2::fetch()
        .await
        .map_err(|e| format!("failed to fetch version manifest: {e}"))?;

    let version = manifest
        .version(&minecraft_version)
        .await
        .map_err(|e| format!("failed to fetch version details: {e}"))?
        .ok_or_else(|| format!("Minecraft version {} not found", minecraft_version))?;

    let java_version = version
        .java_version
        .ok_or_else(|| format!("No Java version info for MC {minecraft_version}"))?;

    Ok(JavaInfo {
        major_version: java_version.major_version,
        label: format!("Java {}", java_version.major_version),
    })
}

// ---------------------------------------------------------------------------
// Directory operations
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn open_directory(path: String) -> Result<(), String> {
    let path = std::path::Path::new(&path);
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.display()));
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open directory: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open directory: {e}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| format!("failed to open directory: {e}"))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Instances directory management
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_instances_dir(
    state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    ensure_manager(&state, &app).await?;
    let guard = state.lock().await;
    let mgr = guard.as_ref().unwrap();
    Ok(mgr.instances_dir().to_string_lossy().to_string())
}

// ---------------------------------------------------------------------------
// Instance filesystem inspection
// ---------------------------------------------------------------------------

fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if ft.is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            } else if ft.is_dir() {
                total += dir_size(&entry.path());
            }
        }
    }
    total
}

fn file_count(path: &Path) -> u64 {
    let mut count = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let ft = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            if ft.is_file() {
                count += 1;
            } else if ft.is_dir() {
                count += file_count(&entry.path());
            }
        }
    }
    count
}

fn count_jars(dir: &Path) -> u64 {
    if !dir.exists() {
        return 0;
    }
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    name.ends_with(".jar") || name.ends_with(".jar.disabled")
                })
                .count() as u64
        })
        .unwrap_or(0)
}

fn count_subdirs(dir: &Path) -> u64 {
    if !dir.exists() {
        return 0;
    }
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                .count() as u64
        })
        .unwrap_or(0)
}

fn count_files_with_ext(dir: &Path, ext: &str) -> u64 {
    if !dir.exists() {
        return 0;
    }
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .to_lowercase()
                        .ends_with(ext)
                })
                .count() as u64
        })
        .unwrap_or(0)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceDetails {
    pub disk_size_bytes: u64,
    pub file_count: u64,
    pub mod_count: u64,
    pub world_count: u64,
    pub screenshot_count: u64,
}

#[tauri::command]
pub async fn get_instance_details(instance_path: String) -> Result<InstanceDetails, String> {
    let root = PathBuf::from(&instance_path);
    if !root.exists() {
        return Err(format!("instance path does not exist: {instance_path}"));
    }
    Ok(InstanceDetails {
        disk_size_bytes: dir_size(&root),
        file_count: file_count(&root),
        mod_count: count_jars(&root.join("mods")),
        world_count: count_subdirs(&root.join("saves")),
        screenshot_count: count_files_with_ext(&root.join("screenshots"), ".png"),
    })
}

// ---------------------------------------------------------------------------
// Mods
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModEntry {
    pub file_name: String,
    pub file_size_bytes: u64,
    pub enabled: bool,
    pub last_modified: String,
}

#[tauri::command]
pub async fn list_instance_mods(instance_path: String) -> Result<Vec<ModEntry>, String> {
    let mods_dir = PathBuf::from(&instance_path).join("mods");
    if !mods_dir.exists() {
        return Ok(vec![]);
    }
    let mut mods = Vec::new();
    let entries = std::fs::read_dir(&mods_dir)
        .map_err(|e| format!("failed to read mods dir: {e}"))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.ends_with(".jar") && !name.ends_with(".jar.disabled") {
            continue;
        }
        let meta = entry.metadata().map_err(|e| format!("metadata error: {e}"))?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();
        mods.push(ModEntry {
            enabled: !name.ends_with(".disabled"),
            file_name: name,
            file_size_bytes: meta.len(),
            last_modified: modified,
        });
    }
    mods.sort_by(|a, b| a.file_name.to_lowercase().cmp(&b.file_name.to_lowercase()));
    Ok(mods)
}

#[tauri::command]
pub async fn toggle_mod(
    instance_path: String,
    file_name: String,
    enabled: bool,
) -> Result<(), String> {
    let mods_dir = PathBuf::from(&instance_path).join("mods");
    let current = mods_dir.join(&file_name);
    if !current.exists() {
        return Err(format!("mod file not found: {file_name}"));
    }
    let new_name = if enabled {
        file_name.strip_suffix(".disabled").unwrap_or(&file_name).to_string()
    } else if file_name.ends_with(".disabled") {
        file_name.clone()
    } else {
        format!("{file_name}.disabled")
    };
    let new_path = mods_dir.join(&new_name);
    std::fs::rename(&current, &new_path)
        .map_err(|e| format!("failed to rename mod: {e}"))
}

#[tauri::command]
pub async fn delete_mod(
    instance_id: i64,
    instance_path: String,
    file_name: String,
    mgr_state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let path = PathBuf::from(&instance_path).join("mods").join(&file_name);
    if !path.exists() {
        return Err(format!("mod file not found: {file_name}"));
    }
    std::fs::remove_file(&path).map_err(|e| format!("failed to delete mod: {e}"))?;

    // Remove from DB (try both the exact name and without .disabled suffix)
    ensure_manager(&mgr_state, &app).await?;
    let guard = mgr_state.lock().await;
    let mgr = guard.as_ref().unwrap();
    let base_name = file_name.strip_suffix(".disabled").unwrap_or(&file_name);
    let _ = mgr.remove_installed_mod(instance_id, &file_name).await;
    if base_name != file_name {
        let _ = mgr.remove_installed_mod(instance_id, base_name).await;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Worlds
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldEntry {
    pub dir_name: String,
    pub world_name: String,
    pub seed: i64,
    pub game_mode: u8,
    pub hardcore: bool,
    pub difficulty: u8,
    pub playtime_ticks: i64,
    pub minecraft_version: String,
    pub size_bytes: u64,
    pub last_modified: String,
    pub has_icon: bool,
}

/// Parse seed from the separate `data/minecraft/world_gen_settings.dat` file
/// used by newer MC versions (26+) where WorldGenSettings was moved out of level.dat.
fn parse_world_gen_seed(world_dir: &Path) -> Option<i64> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    use valence_nbt::Value;

    let wgs_path = world_dir
        .join("data")
        .join("minecraft")
        .join("world_gen_settings.dat");
    if !wgs_path.exists() {
        return None;
    }

    let file = std::fs::File::open(&wgs_path).ok()?;
    let mut decoder = GzDecoder::new(file);
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf).ok()?;

    let (root, _) = valence_nbt::from_binary::<String>(&mut buf.as_slice()).ok()?;

    // Structure: root.data.seed
    let data = match root.get("data")? {
        Value::Compound(c) => c,
        _ => return None,
    };

    match data.get("seed") {
        Some(Value::Long(s)) => Some(*s),
        _ => None,
    }
}

/// Parse level.dat NBT and extract world metadata.
fn parse_level_dat(world_dir: &Path) -> Option<(String, i64, u8, bool, u8, i64, String)> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    use valence_nbt::Value;

    let level_dat = world_dir.join("level.dat");
    if !level_dat.exists() {
        return None;
    }

    let file = std::fs::File::open(&level_dat).ok()?;
    let mut decoder = GzDecoder::new(file);
    let mut buf = Vec::new();
    decoder.read_to_end(&mut buf).ok()?;

    let (root, _) = valence_nbt::from_binary::<String>(&mut buf.as_slice()).ok()?;

    let data = match root.get("Data")? {
        Value::Compound(c) => c,
        _ => return None,
    };

    let world_name = match data.get("LevelName") {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };

    // Seed resolution order:
    // 1. Data.WorldGenSettings.seed (MC 1.16–1.20ish, in level.dat)
    // 2. Data.RandomSeed (legacy MC < 1.16, in level.dat)
    // 3. Separate file: data/minecraft/world_gen_settings.dat (MC 26+)
    //    → root.data.seed
    let seed = if let Some(Value::Compound(wgs)) = data.get("WorldGenSettings") {
        match wgs.get("seed") {
            Some(Value::Long(s)) => *s,
            _ => 0,
        }
    } else if let Some(Value::Long(s)) = data.get("RandomSeed") {
        *s
    } else {
        // Try the separate world_gen_settings.dat file (newest MC versions)
        parse_world_gen_seed(world_dir).unwrap_or(0)
    };

    let game_mode = match data.get("GameType") {
        Some(Value::Int(v)) => *v as u8,
        _ => 0,
    };

    let hardcore = match data.get("hardcore") {
        Some(Value::Byte(v)) => *v != 0,
        _ => false,
    };

    let difficulty = match data.get("Difficulty") {
        Some(Value::Byte(v)) => *v as u8,
        _ => 2,
    };

    let playtime_ticks = match data.get("Time") {
        Some(Value::Long(v)) => *v,
        _ => 0,
    };

    let mc_version = if let Some(Value::Compound(ver)) = data.get("Version") {
        match ver.get("Name") {
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        }
    } else {
        String::new()
    };

    Some((world_name, seed, game_mode, hardcore, difficulty, playtime_ticks, mc_version))
}

#[tauri::command]
pub async fn list_instance_worlds(instance_path: String) -> Result<Vec<WorldEntry>, String> {
    let saves_dir = PathBuf::from(&instance_path).join("saves");
    if !saves_dir.exists() {
        return Ok(vec![]);
    }
    let mut worlds = Vec::new();
    let entries = std::fs::read_dir(&saves_dir)
        .map_err(|e| format!("failed to read saves dir: {e}"))?;
    for entry in entries.flatten() {
        if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            continue;
        }
        let dir_name = entry.file_name().to_string_lossy().to_string();
        let world_path = entry.path();
        let meta = entry.metadata().map_err(|e| format!("metadata error: {e}"))?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();

        let (world_name, seed, game_mode, hardcore, difficulty, playtime_ticks, minecraft_version) =
            parse_level_dat(&world_path).unwrap_or_else(|| {
                (dir_name.clone(), 0, 0, false, 2, 0, String::new())
            });

        let has_icon = world_path.join("icon.png").exists();

        worlds.push(WorldEntry {
            size_bytes: dir_size(&world_path),
            dir_name,
            world_name,
            seed,
            game_mode,
            hardcore,
            difficulty,
            playtime_ticks,
            minecraft_version,
            last_modified: modified,
            has_icon,
        });
    }
    worlds.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    Ok(worlds)
}

#[tauri::command]
pub async fn read_world_icon(
    instance_path: String,
    dir_name: String,
) -> Result<Vec<u8>, String> {
    let path = PathBuf::from(&instance_path)
        .join("saves")
        .join(&dir_name)
        .join("icon.png");
    if !path.exists() {
        return Err("icon.png not found".into());
    }
    std::fs::read(&path).map_err(|e| format!("failed to read icon: {e}"))
}

#[tauri::command]
pub async fn delete_world(instance_path: String, dir_name: String) -> Result<(), String> {
    let path = PathBuf::from(&instance_path).join("saves").join(&dir_name);
    if !path.exists() {
        return Err(format!("world not found: {dir_name}"));
    }
    std::fs::remove_dir_all(&path).map_err(|e| format!("failed to delete world: {e}"))
}

// ---------------------------------------------------------------------------
// Screenshots
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotEntry {
    pub file_name: String,
    pub file_size_bytes: u64,
    pub last_modified: String,
}

#[tauri::command]
pub async fn list_instance_screenshots(
    instance_path: String,
) -> Result<Vec<ScreenshotEntry>, String> {
    let dir = PathBuf::from(&instance_path).join("screenshots");
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut shots = Vec::new();
    let entries =
        std::fs::read_dir(&dir).map_err(|e| format!("failed to read screenshots dir: {e}"))?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.to_lowercase().ends_with(".png") {
            continue;
        }
        let meta = entry.metadata().map_err(|e| format!("metadata error: {e}"))?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();
        shots.push(ScreenshotEntry {
            file_name: name,
            file_size_bytes: meta.len(),
            last_modified: modified,
        });
    }
    shots.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    Ok(shots)
}

#[tauri::command]
pub async fn delete_screenshot(
    instance_path: String,
    file_name: String,
) -> Result<(), String> {
    let path = PathBuf::from(&instance_path)
        .join("screenshots")
        .join(&file_name);
    if !path.exists() {
        return Err(format!("screenshot not found: {file_name}"));
    }
    std::fs::remove_file(&path).map_err(|e| format!("failed to delete screenshot: {e}"))
}

#[tauri::command]
pub async fn read_instance_screenshot(
    instance_path: String,
    file_name: String,
) -> Result<Vec<u8>, String> {
    let path = PathBuf::from(&instance_path)
        .join("screenshots")
        .join(&file_name);
    if !path.exists() {
        return Err(format!("screenshot not found: {file_name}"));
    }
    std::fs::read(&path).map_err(|e| format!("failed to read screenshot: {e}"))
}

// ---------------------------------------------------------------------------
// Logs
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn list_log_files(instance_path: String) -> Result<Vec<String>, String> {
    let dir = PathBuf::from(&instance_path).join("logs");
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut files = Vec::new();
    let entries =
        std::fs::read_dir(&dir).map_err(|e| format!("failed to read logs dir: {e}"))?;
    for entry in entries.flatten() {
        if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            files.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    // Put latest.log first, then sort rest
    files.sort_by(|a, b| {
        if a == "latest.log" {
            std::cmp::Ordering::Less
        } else if b == "latest.log" {
            std::cmp::Ordering::Greater
        } else {
            b.cmp(a)
        }
    });
    Ok(files)
}

#[tauri::command]
pub async fn read_log_file(
    instance_path: String,
    file_name: String,
) -> Result<String, String> {
    let path = PathBuf::from(&instance_path).join("logs").join(&file_name);
    if !path.exists() {
        return Err(format!("log file not found: {file_name}"));
    }
    let content =
        std::fs::read_to_string(&path).map_err(|e| format!("failed to read log file: {e}"))?;
    // Return last 2000 lines to avoid huge payloads
    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(2000);
    Ok(lines[start..].join("\n"))
}

// ---------------------------------------------------------------------------
// File browser
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub last_modified: String,
}

#[tauri::command]
pub async fn list_instance_files(
    instance_path: String,
    sub_path: String,
) -> Result<Vec<FileEntry>, String> {
    let root = PathBuf::from(&instance_path);
    let target = root.join(&sub_path);
    // Security: ensure we don't escape the instance directory
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("failed to canonicalize root: {e}"))?;
    let canonical_target = target
        .canonicalize()
        .map_err(|e| format!("failed to canonicalize target: {e}"))?;
    if !canonical_target.starts_with(&canonical_root) {
        return Err("path traversal not allowed".into());
    }
    if !canonical_target.exists() {
        return Ok(vec![]);
    }
    let mut entries_out = Vec::new();
    let entries = std::fs::read_dir(&canonical_target)
        .map_err(|e| format!("failed to read directory: {e}"))?;
    for entry in entries.flatten() {
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let is_dir = meta.is_dir();
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();
        entries_out.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            is_dir,
            size_bytes: if is_dir { 0 } else { meta.len() },
            last_modified: modified,
        });
    }
    // Directories first, then alphabetical
    entries_out.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries_out)
}

// ---------------------------------------------------------------------------
// Per-instance settings
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct InstanceSettings {
    pub max_memory_mb: Option<u32>,
    pub java_path: Option<String>,
    pub jvm_arguments: Option<String>,
    pub window_mode: Option<String>,
    pub resolution_width: Option<u32>,
    pub resolution_height: Option<u32>,
    pub hide_launcher: Option<bool>,
    pub quit_after_game: Option<bool>,
}

#[tauri::command]
pub async fn get_instance_settings(
    instance_path: String,
) -> Result<InstanceSettings, String> {
    let path = PathBuf::from(&instance_path).join("lodestone_settings.json");
    if !path.exists() {
        return Ok(InstanceSettings::default());
    }
    let data = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read instance settings: {e}"))?;
    serde_json::from_str(&data)
        .map_err(|e| format!("failed to parse instance settings: {e}"))
}

#[tauri::command]
pub async fn save_instance_settings(
    instance_path: String,
    settings: InstanceSettings,
) -> Result<(), String> {
    let path = PathBuf::from(&instance_path).join("lodestone_settings.json");
    let json = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json)
        .map_err(|e| format!("failed to write instance settings: {e}"))
}

// ---------------------------------------------------------------------------
// Mod installation with dependency resolution
// ---------------------------------------------------------------------------

/// Find the best matching version for a project given MC version and loader.
async fn find_matching_version(
    project_id: &str,
    platform: hopper_mc::Platform,
    mc_version: &str,
    loader: &str,
) -> Result<Option<hopper_mc::ProjectVersion>, String> {
    let versions = hopper_mc::get_versions(project_id, platform)
        .await
        .map_err(|e| format!("failed to fetch versions for {project_id}: {e}"))?;

    // Find first version that matches both MC version and loader
    let matching = versions.into_iter().find(|v| {
        v.game_versions.iter().any(|gv| gv == mc_version)
            && v.loaders.iter().any(|l| l.eq_ignore_ascii_case(loader))
    });
    Ok(matching)
}

/// Info about a resolved mod file to download + track in the DB.
struct ResolvedModFile {
    filename: String,
    url: String,
    project_id: String,
    version_id: String,
    project_name: String,
    icon_url: Option<String>,
}

/// Collect all files to download for a mod + its required dependencies.
/// Deduplicates by project_id.
fn resolve_mod_files<'a>(
    project_id: &'a str,
    platform: hopper_mc::Platform,
    mc_version: &'a str,
    loader: &'a str,
    mods_dir: &'a Path,
    resolved: &'a mut std::collections::HashSet<String>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<ResolvedModFile>, String>> + Send + 'a>> {
    Box::pin(async move {
    if resolved.contains(project_id) {
        return Ok(vec![]);
    }
    resolved.insert(project_id.to_string());

    let version = find_matching_version(project_id, platform, mc_version, loader)
        .await?
        .ok_or_else(|| {
            format!("No compatible version found for {project_id} (MC {mc_version}, {loader})")
        })?;

    let mut files = Vec::new();

    // Fetch project info for icon + title
    let project_info = hopper_mc::get_mod(project_id, platform)
        .await
        .ok()
        .flatten();
    let icon_url = project_info.as_ref().and_then(|m| m.base.icon_url.clone());
    let mod_title = project_info.as_ref().map(|m| m.base.title.clone())
        .unwrap_or_else(|| version.name.clone());

    // Get the primary file
    if let Some(file) = version.files.iter().find(|f| f.primary).or(version.files.first()) {
        if let Some(url) = &file.url {
            if !mods_dir.join(&file.filename).exists() {
                files.push(ResolvedModFile {
                    filename: file.filename.clone(),
                    url: url.clone(),
                    project_id: version.project_id.clone(),
                    version_id: version.id.clone(),
                    project_name: mod_title.clone(),
                    icon_url: icon_url.clone(),
                });
            }
        }
    }

    // Resolve required dependencies
    for dep in &version.dependencies {
        if dep.kind != hopper_mc::DependencyKind::Required {
            continue;
        }
        if let Some(dep_project_id) = &dep.project_id {
            match resolve_mod_files(dep_project_id, platform, mc_version, loader, mods_dir, resolved).await {
                Ok(dep_files) => files.extend(dep_files),
                Err(e) => {
                    log::warn!("Failed to resolve dependency {dep_project_id}: {e}");
                }
            }
        }
    }

    Ok(files)
    }) // end Box::pin
}

/// Helper to download resolved mod files and return installed filenames.
async fn download_mod_files(
    files: &[ResolvedModFile],
    mods_dir: &Path,
) -> Result<Vec<String>, String> {
    if files.is_empty() {
        return Ok(vec![]);
    }
    let client = reqwest::Client::new();
    let mut installed = Vec::new();
    for f in files {
        let dest = mods_dir.join(&f.filename);
        let response = client.get(&f.url).send().await
            .map_err(|e| format!("failed to download {}: {e}", f.filename))?;
        if !response.status().is_success() {
            return Err(format!("download failed for {}: HTTP {}", f.filename, response.status()));
        }
        let bytes = response.bytes().await
            .map_err(|e| format!("failed to read {}: {e}", f.filename))?;
        std::fs::write(&dest, &bytes)
            .map_err(|e| format!("failed to write {}: {e}", f.filename))?;
        installed.push(f.filename.clone());
    }
    Ok(installed)
}

#[tauri::command]
pub async fn install_mod(
    instance_id: i64,
    instance_path: String,
    project_id: String,
    platform: String,
    mc_version: String,
    loader: String,
    project_name: Option<String>,
    mgr_state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    let plat = match platform.as_str() {
        "modrinth" => hopper_mc::Platform::Modrinth,
        "curseforge" => hopper_mc::Platform::CurseForge,
        _ => return Err(format!("unsupported platform: {platform}")),
    };

    let mods_dir = PathBuf::from(&instance_path).join("mods");
    std::fs::create_dir_all(&mods_dir)
        .map_err(|e| format!("failed to create mods dir: {e}"))?;

    let mut resolved = std::collections::HashSet::new();
    let files = resolve_mod_files(&project_id, plat, &mc_version, &loader, &mods_dir, &mut resolved).await?;
    let installed = download_mod_files(&files, &mods_dir).await?;

    // Save ALL resolved mods (including dependencies) to DB
    if !files.is_empty() {
        ensure_manager(&mgr_state, &app).await?;
        let guard = mgr_state.lock().await;
        let mgr = guard.as_ref().unwrap();
        for f in &files {
            // Check if already tracked
            let (mr_id, cf_id) = match platform.as_str() {
                "modrinth" => (Some(f.project_id.as_str()), None),
                "curseforge" => (None, Some(f.project_id.as_str())),
                _ => (None, None),
            };
            let existing = mgr.find_installed_mod_by_project(instance_id, mr_id, cf_id).await.ok().flatten();
            if existing.is_none() {
                // Use the user-provided name for the primary mod, version name for deps
                let name = if f.project_id == project_id {
                    project_name.as_deref().unwrap_or(&f.project_name)
                } else {
                    &f.project_name
                };
                let _ = mgr.add_installed_mod(
                    instance_id, &f.filename, mr_id, cf_id,
                    Some(f.version_id.as_str()), Some(name),
                    f.icon_url.as_deref(),
                ).await;
            }
        }
    }

    Ok(installed)
}

#[tauri::command]
pub async fn install_mod_version(
    instance_id: i64,
    instance_path: String,
    version_id: String,
    platform: String,
    mc_version: String,
    loader: String,
    project_name: Option<String>,
    mgr_state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    let plat = match platform.as_str() {
        "modrinth" => hopper_mc::Platform::Modrinth,
        "curseforge" => hopper_mc::Platform::CurseForge,
        _ => return Err(format!("unsupported platform: {platform}")),
    };

    let mods_dir = PathBuf::from(&instance_path).join("mods");
    std::fs::create_dir_all(&mods_dir)
        .map_err(|e| format!("failed to create mods dir: {e}"))?;

    let version = hopper_mc::get_version(&version_id, plat)
        .await
        .map_err(|e| format!("failed to fetch version: {e}"))?
        .ok_or_else(|| format!("version {version_id} not found"))?;

    // Fetch icon for the primary project
    let primary_icon = hopper_mc::get_mod(&version.project_id, plat)
        .await.ok().flatten().and_then(|m| m.base.icon_url);

    let mut files_to_download: Vec<ResolvedModFile> = Vec::new();
    if let Some(file) = version.files.iter().find(|f| f.primary).or(version.files.first()) {
        if let Some(url) = &file.url {
            if !mods_dir.join(&file.filename).exists() {
                files_to_download.push(ResolvedModFile {
                    filename: file.filename.clone(),
                    url: url.clone(),
                    project_id: version.project_id.clone(),
                    version_id: version.id.clone(),
                    project_name: version.name.clone(),
                    icon_url: primary_icon.clone(),
                });
            }
        }
    }

    let mut resolved = std::collections::HashSet::new();
    resolved.insert(version.project_id.clone());
    for dep in &version.dependencies {
        if dep.kind != hopper_mc::DependencyKind::Required { continue; }
        if let Some(dep_project_id) = &dep.project_id {
            match resolve_mod_files(dep_project_id, plat, &mc_version, &loader, &mods_dir, &mut resolved).await {
                Ok(dep_files) => files_to_download.extend(dep_files),
                Err(e) => log::warn!("Failed to resolve dependency {dep_project_id}: {e}"),
            }
        }
    }

    let installed = download_mod_files(&files_to_download, &mods_dir).await?;

    // Save ALL mods (primary + deps) to DB
    if !files_to_download.is_empty() {
        ensure_manager(&mgr_state, &app).await?;
        let guard = mgr_state.lock().await;
        let mgr = guard.as_ref().unwrap();
        for f in &files_to_download {
            let (mr_id, cf_id) = match platform.as_str() {
                "modrinth" => (Some(f.project_id.as_str()), None),
                "curseforge" => (None, Some(f.project_id.as_str())),
                _ => (None, None),
            };
            let existing = mgr.find_installed_mod_by_project(instance_id, mr_id, cf_id).await.ok().flatten();
            if let Some(rec) = existing {
                let _ = mgr.update_installed_mod(rec.id, &f.filename, Some(&f.version_id)).await;
            } else {
                let name = if f.project_id == version.project_id {
                    project_name.as_deref().unwrap_or(&f.project_name)
                } else {
                    &f.project_name
                };
                let _ = mgr.add_installed_mod(
                    instance_id, &f.filename, mr_id, cf_id,
                    Some(f.version_id.as_str()), Some(name),
                    f.icon_url.as_deref(),
                ).await;
            }
        }
    }

    Ok(installed)
}

/// Replace an installed mod version: delete the old file, download the new version.
#[tauri::command]
pub async fn replace_mod_version(
    instance_id: i64,
    instance_path: String,
    old_file_name: String,
    version_id: String,
    platform: String,
    mc_version: String,
    loader: String,
    mgr_state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    let plat = match platform.as_str() {
        "modrinth" => hopper_mc::Platform::Modrinth,
        "curseforge" => hopper_mc::Platform::CurseForge,
        _ => return Err(format!("unsupported platform: {platform}")),
    };

    let mods_dir = PathBuf::from(&instance_path).join("mods");

    // Delete old file
    let old_path = mods_dir.join(&old_file_name);
    if old_path.exists() {
        std::fs::remove_file(&old_path).map_err(|e| format!("failed to delete old mod: {e}"))?;
    }
    // Also try .disabled variant
    let old_disabled = mods_dir.join(format!("{old_file_name}.disabled"));
    if old_disabled.exists() {
        let _ = std::fs::remove_file(&old_disabled);
    }

    let version = hopper_mc::get_version(&version_id, plat)
        .await
        .map_err(|e| format!("failed to fetch version: {e}"))?
        .ok_or_else(|| format!("version {version_id} not found"))?;

    let replace_icon = hopper_mc::get_mod(&version.project_id, plat)
        .await.ok().flatten().and_then(|m| m.base.icon_url);

    let mut files_to_download: Vec<ResolvedModFile> = Vec::new();
    if let Some(file) = version.files.iter().find(|f| f.primary).or(version.files.first()) {
        if let Some(url) = &file.url {
            files_to_download.push(ResolvedModFile {
                filename: file.filename.clone(),
                url: url.clone(),
                project_id: version.project_id.clone(),
                version_id: version.id.clone(),
                project_name: version.name.clone(),
                icon_url: replace_icon.clone(),
            });
        }
    }

    // Also resolve deps
    let mut resolved = std::collections::HashSet::new();
    resolved.insert(version.project_id.clone());
    for dep in &version.dependencies {
        if dep.kind != hopper_mc::DependencyKind::Required { continue; }
        if let Some(dep_project_id) = &dep.project_id {
            match resolve_mod_files(dep_project_id, plat, &mc_version, &loader, &mods_dir, &mut resolved).await {
                Ok(dep_files) => files_to_download.extend(dep_files),
                Err(e) => log::warn!("Failed to resolve dependency {dep_project_id}: {e}"),
            }
        }
    }

    let installed = download_mod_files(&files_to_download, &mods_dir).await?;

    // Update DB records for all files
    if !files_to_download.is_empty() {
        ensure_manager(&mgr_state, &app).await?;
        let guard = mgr_state.lock().await;
        let mgr = guard.as_ref().unwrap();
        for f in &files_to_download {
            let (mr_id, cf_id) = match platform.as_str() {
                "modrinth" => (Some(f.project_id.as_str()), None),
                "curseforge" => (None, Some(f.project_id.as_str())),
                _ => (None, None),
            };
            let existing = mgr.find_installed_mod_by_project(instance_id, mr_id, cf_id).await.ok().flatten();
            if let Some(rec) = existing {
                let _ = mgr.update_installed_mod(rec.id, &f.filename, Some(&f.version_id)).await;
            } else {
                let _ = mgr.add_installed_mod(
                    instance_id, &f.filename, mr_id, cf_id,
                    Some(f.version_id.as_str()), Some(&f.project_name),
                    f.icon_url.as_deref(),
                ).await;
            }
        }
    }

    Ok(installed)
}

/// Get installed mod metadata for an instance (from DB).
#[tauri::command]
pub async fn get_installed_mods_info(
    instance_id: i64,
    mgr_state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<Vec<serde_json::Value>, String> {
    ensure_manager(&mgr_state, &app).await?;
    let guard = mgr_state.lock().await;
    let mgr = guard.as_ref().unwrap();
    let records = mgr.list_installed_mods(instance_id).await
        .map_err(|e| format!("failed to list installed mods: {e}"))?;
    serde_json::to_value(&records)
        .map(|v| match v { serde_json::Value::Array(a) => a, _ => vec![] })
        .map_err(|e| format!("serialization error: {e}"))
}
