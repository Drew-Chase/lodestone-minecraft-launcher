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
pub async fn delete_mod(instance_path: String, file_name: String) -> Result<(), String> {
    let path = PathBuf::from(&instance_path).join("mods").join(&file_name);
    if !path.exists() {
        return Err(format!("mod file not found: {file_name}"));
    }
    std::fs::remove_file(&path).map_err(|e| format!("failed to delete mod: {e}"))
}

// ---------------------------------------------------------------------------
// Worlds
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldEntry {
    pub dir_name: String,
    pub size_bytes: u64,
    pub last_modified: String,
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
        let meta = entry.metadata().map_err(|e| format!("metadata error: {e}"))?;
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();
        worlds.push(WorldEntry {
            size_bytes: dir_size(&entry.path()),
            dir_name,
            last_modified: modified,
        });
    }
    worlds.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));
    Ok(worlds)
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
