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

async fn ensure_manager(
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
