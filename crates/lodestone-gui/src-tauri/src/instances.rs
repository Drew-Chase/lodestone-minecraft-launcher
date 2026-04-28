use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
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
// Instance images (icon.png / banner.png)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn read_instance_image(
    instance_path: String,
    image_name: String,
) -> Result<Vec<u8>, String> {
    if image_name != "icon.png" && image_name != "banner.png" {
        return Err("invalid image name".into());
    }
    let path = PathBuf::from(&instance_path).join(&image_name);
    if !path.exists() {
        return Err(format!("{image_name} not found"));
    }
    std::fs::read(&path).map_err(|e| format!("failed to read {image_name}: {e}"))
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

// ---------------------------------------------------------------------------
// Modpack import & installation
// ---------------------------------------------------------------------------

/// Progress payload that matches the frontend's `InstallProgress` interface,
/// so modpack installs appear in the Downloads popover automatically.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallProgressPayload {
    instance_id: i64,
    instance_name: String,
    stage: String,
    stage_label: String,
    progress: f64,
    files_done: usize,
    files_total: usize,
}

fn emit_install_progress(app: &tauri::AppHandle, payload: &InstallProgressPayload) {
    let _ = app.emit("install-progress", payload);
}

/// Emitted when a modpack install finishes so the frontend can move it
/// from "installing" to "completed" without waiting for `instance-started`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallCompletedPayload {
    instance_id: i64,
    instance_name: String,
}

fn emit_install_completed(app: &tauri::AppHandle, payload: &InstallCompletedPayload) {
    let _ = app.emit("install-completed", payload);
}

/// Optional metadata about the modpack sourced from the platform API.
/// Used to download the icon/banner and to populate mod DB records.
struct PackMeta {
    icon_url: Option<String>,
    banner_url: Option<String>,
    platform: Option<hopper_mc::Platform>,
}

/// Download an image URL to a local file path, ignoring errors silently.
async fn download_image(client: &reqwest::Client, url: &str, dest: &Path) {
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(bytes) = resp.bytes().await {
                let _ = std::fs::write(dest, &bytes);
            }
        }
        _ => {}
    }
}

/// Extract Modrinth project ID and version ID from a CDN download URL.
/// URLs follow the pattern: `https://cdn.modrinth.com/data/{project_id}/versions/{version_id}/{filename}`
fn parse_modrinth_cdn_url(url: &str) -> Option<(String, String)> {
    let url = url.strip_prefix("https://cdn.modrinth.com/data/")?;
    let mut parts = url.splitn(4, '/');
    let project_id = parts.next()?;
    let _versions = parts.next().filter(|s| *s == "versions")?;
    let version_id = parts.next()?;
    Some((project_id.to_string(), version_id.to_string()))
}

/// Shared logic for installing a modpack from a local archive file.
async fn install_modpack_from_archive(
    archive_path: &Path,
    state: &InstanceManagerState,
    app: &tauri::AppHandle,
    meta: Option<PackMeta>,
) -> Result<InstanceConfig, String> {
    // 1. Parse the archive
    let file = std::fs::File::open(archive_path)
        .map_err(|e| format!("failed to open archive: {e}"))?;

    let mut manifest = hopper_mc::parse_modpack(&file)
        .map_err(|e| format!("failed to parse modpack: {e}"))?;

    let pack_name = manifest.name.clone();

    // 2. Resolve CurseForge file URLs if needed
    if manifest.source == hopper_mc::ModpackSource::CurseForge {
        let unresolved = manifest.files.iter().filter(|f| f.download_urls.is_empty()).count();
        if unresolved > 0 {
            let cf = hopper_mc::CurseForgeProvider::shared();
            cf.resolve_pack_files(&mut manifest.files)
                .await
                .map_err(|e| format!("failed to resolve CurseForge files: {e}"))?;
        }
    }

    // 3. Create instance

    let loader = LoaderType::parse(&manifest.loader).unwrap_or(LoaderType::Vanilla);
    let loader_version = if manifest.loader_version.is_empty() {
        None
    } else {
        Some(manifest.loader_version.clone())
    };

    let params = CreateInstanceParams {
        name: manifest.name.clone(),
        minecraft_version: manifest.minecraft_version.clone(),
        loader,
        loader_version,
        java_version: None,
    };

    ensure_manager(state, app).await?;
    let instance = {
        let guard = state.lock().await;
        let mgr = guard.as_ref().unwrap();
        mgr.create(params)
            .await
            .map_err(|e| format!("failed to create instance: {e}"))?
    };

    let real_id = instance.id;
    let instance_dir = PathBuf::from(&instance.instance_path);

    // 3b. Download icon and banner images if available
    let http_client = reqwest::Client::new();
    if let Some(ref meta) = meta {
        if let Some(ref icon) = meta.icon_url {
            download_image(&http_client, icon, &instance_dir.join("icon.png")).await;
        }
        if let Some(ref banner) = meta.banner_url {
            download_image(&http_client, banner, &instance_dir.join("banner.png")).await;
        }
    }

    // 4. Download mod files
    let downloadable: Vec<_> = manifest
        .files
        .iter()
        .filter(|f| !f.download_urls.is_empty() && f.required)
        .collect();
    let total = downloadable.len();

    if total > 0 {
        let client = &http_client;
        let sem = Arc::new(tokio::sync::Semaphore::new(8));

        for (i, pack_file) in downloadable.iter().enumerate() {
            let file_name = pack_file
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&pack_file.path);

            let progress_frac = 0.1 + 0.8 * ((i as f64) / (total as f64));
            emit_install_progress(app, &InstallProgressPayload {
                instance_id: real_id,
                instance_name: pack_name.clone(),
                stage: "downloadingMods".to_string(),
                stage_label: format!("Downloading {file_name}..."),
                progress: progress_frac,
                files_done: i,
                files_total: total,
            });

            let dest = instance_dir.join(&pack_file.path);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create directory: {e}"))?;
            }

            let _permit = sem.acquire().await.map_err(|e| format!("semaphore error: {e}"))?;

            let mut downloaded = false;
            for url in &pack_file.download_urls {
                match client.get(url).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        let bytes = resp.bytes().await
                            .map_err(|e| format!("failed to read {file_name}: {e}"))?;
                        std::fs::write(&dest, &bytes)
                            .map_err(|e| format!("failed to write {file_name}: {e}"))?;
                        downloaded = true;
                        break;
                    }
                    Ok(resp) => {
                        log::warn!("download failed for {file_name} from {url}: HTTP {}", resp.status());
                    }
                    Err(e) => {
                        log::warn!("download error for {file_name} from {url}: {e}");
                    }
                }
            }
            if !downloaded {
                log::warn!("could not download {file_name} from any URL");
            }
        }
    }

    // 5. Extract overrides
    emit_install_progress(app, &InstallProgressPayload {
        instance_id: real_id,
        instance_name: pack_name.clone(),
        stage: "extractingOverrides".to_string(),
        stage_label: "Extracting overrides...".to_string(),
        progress: 0.92,
        files_done: total,
        files_total: total,
    });

    let override_file = std::fs::File::open(archive_path)
        .map_err(|e| format!("failed to reopen archive for overrides: {e}"))?;
    let _ = hopper_mc::extract_overrides(&override_file, &instance_dir, true)
        .map_err(|e| format!("failed to extract overrides: {e}"))?;

    // 6. Track mods in DB with proper platform IDs, names, and icons.
    //    Fetch mod metadata from the platform API concurrently.
    {
        // Build a list of (file_name, mr_id, cf_id, version_id) tuples
        struct ModDbEntry {
            file_name: String,
            mr_id: Option<String>,
            cf_id: Option<String>,
            version_id: Option<String>,
            project_name: String,
            icon_url: Option<String>,
        }

        let mut entries: Vec<ModDbEntry> = Vec::new();
        for pack_file in &manifest.files {
            if !pack_file.required {
                continue;
            }
            let file_name = pack_file
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&pack_file.path)
                .to_string();

            let (mr_id, cf_id, version_id) = match manifest.source {
                hopper_mc::ModpackSource::Modrinth => {
                    let parsed = pack_file.download_urls.first()
                        .and_then(|url| parse_modrinth_cdn_url(url));
                    match parsed {
                        Some((pid, vid)) => (Some(pid), None, Some(vid)),
                        None => (None, None, None),
                    }
                }
                hopper_mc::ModpackSource::CurseForge => {
                    let cf = pack_file.project_id.clone();
                    let vid = pack_file.file_id.clone();
                    (None, cf, vid)
                }
            };

            // Fallback name from filename
            let fallback_name = file_name
                .rsplit_once('.')
                .map(|(name, _)| name)
                .unwrap_or(&file_name)
                .to_string();

            entries.push(ModDbEntry {
                file_name,
                mr_id,
                cf_id,
                version_id,
                project_name: fallback_name,
                icon_url: None,
            });
        }

        // Fetch mod metadata from platform APIs concurrently
        let sem = Arc::new(tokio::sync::Semaphore::new(8));
        let mut handles = Vec::new();
        for (idx, entry) in entries.iter().enumerate() {
            let project_id = entry.mr_id.clone().or_else(|| entry.cf_id.clone());
            let Some(pid) = project_id else { continue };
            let platform = if entry.mr_id.is_some() {
                hopper_mc::Platform::Modrinth
            } else {
                hopper_mc::Platform::CurseForge
            };
            let sem = Arc::clone(&sem);
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let info = hopper_mc::get_mod(&pid, platform).await.ok().flatten();
                (idx, info)
            }));
        }
        for handle in handles {
            if let Ok((idx, Some(mod_info))) = handle.await {
                entries[idx].project_name = mod_info.base.title;
                entries[idx].icon_url = mod_info.base.icon_url;
            }
        }

        // Save all entries to DB
        let guard = state.lock().await;
        let mgr = guard.as_ref().unwrap();
        for entry in &entries {
            let _ = mgr
                .add_installed_mod(
                    instance.id,
                    &entry.file_name,
                    entry.mr_id.as_deref(),
                    entry.cf_id.as_deref(),
                    entry.version_id.as_deref(),
                    Some(&entry.project_name),
                    entry.icon_url.as_deref(),
                )
                .await;
        }
    }

    // 7. Save to recent-imports (DB + archive copy)
    save_recent_import(state, app, archive_path, &manifest.name, &manifest.source).await?;

    // 8. Signal completion — progress=1.0 then install-completed event
    emit_install_progress(app, &InstallProgressPayload {
        instance_id: real_id,
        instance_name: pack_name.clone(),
        stage: "complete".to_string(),
        stage_label: "Install complete!".to_string(),
        progress: 1.0,
        files_done: total,
        files_total: total,
    });
    emit_install_completed(app, &InstallCompletedPayload {
        instance_id: real_id,
        instance_name: pack_name,
    });

    Ok(instance)
}

// ---------------------------------------------------------------------------
// Recent imports tracking (stored in SQLite, archives in recent-imports dir)
// ---------------------------------------------------------------------------

fn recent_imports_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    let dir = data_dir.join("recent-imports");
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("failed to create recent-imports dir: {e}"))?;
    Ok(dir)
}

/// Compute SHA-256 hash of a file for deduplication.
fn hash_file(path: &Path) -> Result<String, String> {
    use sha2::{Sha256, Digest};
    let bytes = std::fs::read(path)
        .map_err(|e| format!("failed to read file for hashing: {e}"))?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

async fn save_recent_import(
    state: &InstanceManagerState,
    app: &tauri::AppHandle,
    archive_path: &Path,
    name: &str,
    source: &hopper_mc::ModpackSource,
) -> Result<(), String> {
    let dir = recent_imports_dir(app)?;
    let file_hash = hash_file(archive_path)?;

    let size_bytes = std::fs::metadata(archive_path)
        .map(|m| m.len() as i64)
        .unwrap_or(0);

    let original_name = archive_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "modpack.zip".to_string());

    // Use the hash as part of the filename so duplicate packs reuse the same file
    let dest_name = format!("{}_{}", &file_hash[..12], original_name);
    let dest = dir.join(&dest_name);

    // Only copy if not already present
    if !dest.exists() {
        std::fs::copy(archive_path, &dest)
            .map_err(|e| format!("failed to copy archive to recent-imports: {e}"))?;
    }

    let source_str = match source {
        hopper_mc::ModpackSource::Modrinth => "Modrinth",
        hopper_mc::ModpackSource::CurseForge => "CurseForge",
    };

    let guard = state.lock().await;
    let mgr = guard.as_ref().unwrap();
    mgr.add_recent_import(&file_hash, name, source_str, size_bytes, &dest_name)
        .await
        .map_err(|e| format!("failed to save recent import: {e}"))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Modpack Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn import_modpack(
    file_path: String,
    state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<InstanceConfig, String> {
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err(format!("file not found: {file_path}"));
    }

    install_modpack_from_archive(&path, &state, &app, None).await
}

#[tauri::command]
pub async fn install_modpack_from_discover(
    project_id: String,
    version_id: String,
    platform: String,
    state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<InstanceConfig, String> {
    let plat = match platform.as_str() {
        "modrinth" => hopper_mc::Platform::Modrinth,
        "curseforge" => hopper_mc::Platform::CurseForge,
        _ => return Err(format!("unsupported platform: {platform}")),
    };

    // Fetch pack info for icon and gallery (banner)
    let pack_info = hopper_mc::get_pack(&project_id, plat).await.ok().flatten();
    let meta = pack_info.as_ref().map(|p| PackMeta {
        icon_url: p.base.icon_url.clone(),
        banner_url: p.base.gallery.first().cloned(),
        platform: Some(plat),
    });

    // Fetch the version to get the download URL
    let version = hopper_mc::get_version(&version_id, plat)
        .await
        .map_err(|e| format!("failed to fetch version: {e}"))?
        .ok_or_else(|| format!("version {version_id} not found"))?;

    let file = version.files.iter().find(|f| f.primary)
        .or(version.files.first())
        .ok_or("no files in version")?;

    let url = file.url.as_ref()
        .ok_or("modpack file has no download URL (restricted distribution)")?;

    // Download to temp directory
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    let tmp_dir = data_dir.join("tmp");
    std::fs::create_dir_all(&tmp_dir)
        .map_err(|e| format!("failed to create tmp dir: {e}"))?;

    let tmp_path = tmp_dir.join(&file.filename);

    let client = reqwest::Client::new();
    let resp = client.get(url).send().await
        .map_err(|e| format!("failed to download modpack: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("download failed: HTTP {}", resp.status()));
    }
    let bytes = resp.bytes().await
        .map_err(|e| format!("failed to read modpack: {e}"))?;
    std::fs::write(&tmp_path, &bytes)
        .map_err(|e| format!("failed to write temp file: {e}"))?;

    let result = install_modpack_from_archive(&tmp_path, &state, &app, meta).await;

    // Clean up temp file
    let _ = std::fs::remove_file(&tmp_path);

    result
}

#[tauri::command]
pub async fn list_recent_imports(
    state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<Vec<lodestone_core::instance_manager::RecentImportRecord>, String> {
    ensure_manager(&state, &app).await?;
    let guard = state.lock().await;
    let mgr = guard.as_ref().unwrap();
    mgr.list_recent_imports()
        .await
        .map_err(|e| format!("failed to list recent imports: {e}"))
}

#[tauri::command]
pub async fn reimport_modpack(
    import_id: i64,
    state: tauri::State<'_, InstanceManagerState>,
    app: tauri::AppHandle,
) -> Result<InstanceConfig, String> {
    ensure_manager(&state, &app).await?;
    let entry = {
        let guard = state.lock().await;
        let mgr = guard.as_ref().unwrap();
        mgr.get_recent_import(import_id)
            .await
            .map_err(|e| format!("failed to get recent import: {e}"))?
            .ok_or_else(|| format!("recent import not found: {import_id}"))?
    };

    let dir = recent_imports_dir(&app)?;
    let archive_path = dir.join(&entry.file_name);

    if !archive_path.exists() {
        return Err(format!("archive file no longer exists: {}", entry.file_name));
    }

    install_modpack_from_archive(&archive_path, &state, &app, None).await
}
