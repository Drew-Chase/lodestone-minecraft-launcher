use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use simple_download_utility::{DownloadProgress, MultiDownloadProgress};
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

use lodestone_core::instance::LoaderType;
use minecraft_modloaders::fabric::FabricModLoader;
use minecraft_modloaders::forge::ForgeModLoader;
use minecraft_modloaders::ModLoader;
use piston_mc::java::JavaManifest;
use piston_mc::manifest_v2::ManifestV2;
use piston_mc::version_manifest::LibraryItemDownloader;

use crate::auth::{AuthState, UserSession};
use crate::instances::InstanceManagerState;

/// Tracks child processes of running Minecraft instances.
pub type RunningInstances = Arc<Mutex<HashMap<i64, tokio::process::Child>>>;

// ---------------------------------------------------------------------------
// Events emitted to the frontend
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub instance_id: i64,
    pub instance_name: String,
    pub stage: String,
    pub stage_label: String,
    pub progress: f32,
    pub files_done: usize,
    pub files_total: usize,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn emit_progress(app: &tauri::AppHandle, p: &InstallProgress) {
    let _ = app.emit("install-progress", p);
}

/// Resolve the Java binary path using the component name from the MC version manifest.
/// The component (e.g. "java-runtime-delta", "java-runtime-epsilon") maps directly
/// to a field on piston-mc's Runtimes struct. Downloads if not already installed.
async fn ensure_java(
    app: &tauri::AppHandle,
    instance_id: i64,
    instance_name: &str,
    java_component: &str,
    java_major: u8,
) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir: {e}"))?;
    let java_dir = data_dir.join("java");

    // Check if this component is already installed
    let install_dir = java_dir.join(java_component);
    let bin_dir = install_dir.join("bin");
    let java_exe = if cfg!(windows) {
        bin_dir.join("java.exe")
    } else {
        bin_dir.join("java")
    };

    if java_exe.exists() {
        return Ok(java_exe);
    }

    // Download the runtime from Mojang
    emit_progress(app, &InstallProgress {
        instance_id,
        instance_name: instance_name.to_string(),
        stage: "java".into(),
        stage_label: format!("Downloading Java {java_major} runtime..."),
        progress: 0.0,
        files_done: 0,
        files_total: 0,
    });

    let manifest = JavaManifest::fetch()
        .await
        .map_err(|e| format!("failed to fetch Java manifest: {e}"))?;

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    let platform = &manifest.windows_x64;
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    let platform = &manifest.windows_x86;
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    let platform = &manifest.macos;
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    let platform = &manifest.macos_arm64;
    #[cfg(target_os = "linux")]
    let platform = &manifest.linux;

    // Select the runtime list by component name from the manifest
    let runtime_list = match java_component {
        "jre-legacy" => &platform.legacy,
        "java-runtime-alpha" => &platform.alpha,
        "java-runtime-beta" => &platform.beta,
        "java-runtime-gamma" => &platform.gamma,
        "java-runtime-gamma-snapshot" => &platform.gamma_snapshot,
        "java-runtime-delta" => &platform.delta,
        "java-runtime-epsilon" => &platform.epsilon,
        _ => &platform.delta, // fallback
    };

    let runtime = runtime_list
        .first()
        .ok_or_else(|| format!("No Mojang Java runtime for component '{java_component}'"))?;

    std::fs::create_dir_all(&install_dir)
        .map_err(|e| format!("failed to create java dir: {e}"))?;

    let (sender, mut receiver) = tokio::sync::mpsc::channel::<MultiDownloadProgress>(32);
    let app_clone = app.clone();
    let name_clone = instance_name.to_string();
    let monitor = tokio::spawn(async move {
        while let Some(msg) = receiver.recv().await {
            let prog = if msg.files_total > 0 {
                msg.files_downloaded as f32 / msg.files_total as f32
            } else {
                0.0
            };
            emit_progress(&app_clone, &InstallProgress {
                instance_id,
                instance_name: name_clone.clone(),
                stage: "java".into(),
                stage_label: format!("Downloading Java {java_major}..."),
                progress: prog,
                files_done: msg.files_downloaded,
                files_total: msg.files_total,
            });
        }
    });

    runtime
        .install(&install_dir, 20, Some(sender))
        .await
        .map_err(|e| format!("failed to install Java {java_major}: {e}"))?;
    let _ = monitor.await;

    if !java_exe.exists() {
        return Err(format!(
            "Java {java_major} executable not found after install at {}",
            java_exe.display()
        ));
    }

    Ok(java_exe)
}

/// Info returned by ensure_game_files needed for launching.
struct GameFiles {
    asset_index: String,
    client_jar: PathBuf,
    main_class: String,
    java_major: u8,
    java_component: String,
}

/// Download the Minecraft client jar, libraries, and assets.
/// Cached version info saved to instance directory after first install.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CachedVersionInfo {
    asset_index: String,
    main_class: String,
    java_major: u8,
    java_component: String,
}

async fn ensure_game_files(
    app: &tauri::AppHandle,
    instance_id: i64,
    instance_name: &str,
    mc_version: &str,
    instance_path: &Path,
) -> Result<GameFiles, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir: {e}"))?;
    let assets_dir = data_dir.join("assets");
    let client_jar = instance_path.join("client.jar");
    let library_dir = instance_path.join("libraries");
    let libraries_marker = library_dir.join(format!(".lodestone-{mc_version}"));
    let asset_index_dir = assets_dir.join("indexes");
    let version_cache_path = instance_path.join(".lodestone-version-info.json");

    // If everything is already installed, use cached version info to avoid network fetch
    if client_jar.exists() && libraries_marker.exists() {
        if let Ok(data) = std::fs::read_to_string(&version_cache_path) {
            if let Ok(cached) = serde_json::from_str::<CachedVersionInfo>(&data) {
                let asset_index_file = asset_index_dir.join(format!("{}.json", cached.asset_index));
                if asset_index_file.exists() {
                    return Ok(GameFiles {
                        asset_index: cached.asset_index,
                        client_jar,
                        main_class: cached.main_class,
                        java_major: cached.java_major,
                        java_component: cached.java_component,
                    });
                }
            }
        }
    }

    std::fs::create_dir_all(&assets_dir).map_err(|e| format!("mkdir assets: {e}"))?;
    std::fs::create_dir_all(&library_dir).map_err(|e| format!("mkdir libraries: {e}"))?;

    let manifest = ManifestV2::fetch()
        .await
        .map_err(|e| format!("failed to fetch MC manifest: {e}"))?;
    let version = manifest
        .version(mc_version)
        .await
        .map_err(|e| format!("failed to fetch version: {e}"))?
        .ok_or_else(|| format!("MC version {mc_version} not found"))?;

    let asset_index = version.assets.clone();
    let main_class = version.main_class.clone();
    let java_major = version
        .java_version
        .as_ref()
        .map(|jv| jv.major_version)
        .unwrap_or(17);
    let java_component = version
        .java_version
        .as_ref()
        .map(|jv| jv.component.clone())
        .unwrap_or_else(|| "java-runtime-gamma".to_string());

    // Download client jar using piston-mc's built-in method
    if !client_jar.exists() {
        emit_progress(app, &InstallProgress {
            instance_id,
            instance_name: instance_name.to_string(),
            stage: "client".into(),
            stage_label: "Downloading Minecraft client...".into(),
            progress: 0.0,
            files_done: 0,
            files_total: 1,
        });

        let (sender, mut receiver) = tokio::sync::mpsc::channel::<DownloadProgress>(32);
        let app_clone = app.clone();
        let name_clone = instance_name.to_string();
        let monitor = tokio::spawn(async move {
            while let Some(msg) = receiver.recv().await {
                let prog = if msg.bytes_to_download > 0 {
                    msg.bytes_downloaded as f32 / msg.bytes_to_download as f32
                } else {
                    0.0
                };
                emit_progress(&app_clone, &InstallProgress {
                    instance_id,
                    instance_name: name_clone.clone(),
                    stage: "client".into(),
                    stage_label: "Downloading Minecraft client...".into(),
                    progress: prog,
                    files_done: if prog >= 1.0 { 1 } else { 0 },
                    files_total: 1,
                });
            }
        });

        version
            .download_client(&client_jar, true, Some(sender))
            .await
            .map_err(|e| format!("failed to download client jar: {e}"))?;
        let _ = monitor.await;
    }

    // Download libraries (skip if already completed for this MC version)
    let libraries_marker = library_dir.join(format!(".lodestone-{mc_version}"));
    if !libraries_marker.exists() {
        emit_progress(app, &InstallProgress {
            instance_id,
            instance_name: instance_name.to_string(),
            stage: "libraries".into(),
            stage_label: "Downloading libraries...".into(),
            progress: 0.0,
            files_done: 0,
            files_total: 0,
        });

        let (sender, mut receiver) = tokio::sync::mpsc::channel::<MultiDownloadProgress>(32);
        let app_clone = app.clone();
        let name_clone = instance_name.to_string();
        let monitor = tokio::spawn(async move {
            while let Some(msg) = receiver.recv().await {
                let prog = if msg.files_total > 0 {
                    msg.files_downloaded as f32 / msg.files_total as f32
                } else {
                    0.0
                };
                emit_progress(&app_clone, &InstallProgress {
                    instance_id,
                    instance_name: name_clone.clone(),
                    stage: "libraries".into(),
                    stage_label: "Downloading libraries...".into(),
                    progress: prog,
                    files_done: msg.files_downloaded,
                    files_total: msg.files_total,
                });
            }
        });

        let http = reqwest::Client::new();
        version
            .libraries
            .download_with_client(&http, &library_dir, 50, Some(sender))
            .await
            .map_err(|e| format!("failed to download libraries: {e}"))?;
        let _ = monitor.await;

        // Write marker so we skip next time
        let _ = std::fs::write(&libraries_marker, mc_version);
    }

    // Download assets (skip if asset index already exists)
    let asset_index_file = assets_dir.join("indexes").join(format!("{asset_index}.json"));
    if !asset_index_file.exists() {
        emit_progress(app, &InstallProgress {
            instance_id,
            instance_name: instance_name.to_string(),
            stage: "assets".into(),
            stage_label: "Downloading assets...".into(),
            progress: 0.0,
            files_done: 0,
            files_total: 0,
        });

        let (sender, mut receiver) = tokio::sync::mpsc::channel::<MultiDownloadProgress>(32);
        let app_clone = app.clone();
        let name_clone = instance_name.to_string();
        let monitor = tokio::spawn(async move {
            while let Some(msg) = receiver.recv().await {
                let prog = if msg.files_total > 0 {
                    msg.files_downloaded as f32 / msg.files_total as f32
                } else {
                    0.0
                };
                emit_progress(&app_clone, &InstallProgress {
                    instance_id,
                    instance_name: name_clone.clone(),
                    stage: "assets".into(),
                    stage_label: format!("Downloading assets ({}/{})", msg.files_downloaded, msg.files_total),
                    progress: prog,
                    files_done: msg.files_downloaded,
                    files_total: msg.files_total,
                });
            }
        });

        let mut assets = version
            .assets()
            .await
            .map_err(|e| format!("failed to fetch asset index: {e}"))?;
        assets
            .download(&assets_dir, 100, Some(sender))
            .await
            .map_err(|e| format!("failed to download assets: {e}"))?;
        let _ = monitor.await;
    }

    // Cache version info so subsequent launches skip the manifest fetch
    let cached = CachedVersionInfo {
        asset_index: asset_index.clone(),
        main_class: main_class.clone(),
        java_major,
        java_component: java_component.clone(),
    };
    if let Ok(json) = serde_json::to_string_pretty(&cached) {
        let _ = std::fs::write(&version_cache_path, json);
    }

    Ok(GameFiles { asset_index, client_jar, main_class, java_major, java_component })
}

/// Build a vanilla Minecraft launch command (no mod loader).
#[allow(clippy::too_many_arguments)]
fn build_vanilla_command(
    java_path: &Path,
    instance_path: &Path,
    client_jar: &Path,
    mc_version: &str,
    main_class: &str,
    assets_dir: &Path,
    asset_index: &str,
    jvm_args: &[&str],
    username: &str,
    uuid: &str,
    access_token: &str,
) -> Result<std::process::Command, String> {
    let abs_java = dunce::canonicalize(java_path)
        .map_err(|e| format!("canonicalize java: {e}"))?;
    let abs_instance = dunce::canonicalize(instance_path)
        .map_err(|e| format!("canonicalize instance: {e}"))?;
    let abs_client = dunce::canonicalize(client_jar)
        .map_err(|e| format!("canonicalize client: {e}"))?;
    let abs_assets = dunce::canonicalize(assets_dir)
        .map_err(|e| format!("canonicalize assets: {e}"))?;

    let libraries_dir = abs_instance.join("libraries");
    let mut classpath_entries: Vec<String> = Vec::new();

    if libraries_dir.exists() {
        collect_jars_recursive(&libraries_dir, &mut classpath_entries)
            .map_err(|e| format!("collect jars: {e}"))?;
    }
    classpath_entries.push(abs_client.display().to_string());

    let sep = if cfg!(windows) { ";" } else { ":" };
    let classpath = classpath_entries.join(sep);

    let mut cmd = std::process::Command::new(&abs_java);
    cmd.current_dir(&abs_instance);

    for arg in jvm_args {
        cmd.arg(arg);
    }

    cmd.arg("-cp").arg(&classpath);
    cmd.arg(main_class);
    cmd.arg("--version").arg(mc_version)
        .arg("--gameDir").arg(&abs_instance)
        .arg("--assetsDir").arg(&abs_assets)
        .arg("--assetIndex").arg(asset_index)
        .arg("--accessToken").arg(access_token)
        .arg("--username").arg(username)
        .arg("--uuid").arg(uuid);

    Ok(cmd)
}

fn collect_jars_recursive(dir: &Path, entries: &mut Vec<String>) -> Result<(), String> {
    let read = std::fs::read_dir(dir).map_err(|e| format!("read dir: {e}"))?;
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jars_recursive(&path, entries)?;
        } else if path.extension().map(|e| e == "jar").unwrap_or(false) {
            entries.push(path.display().to_string());
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn launch_instance(
    instance_id: i64,
    mgr_state: tauri::State<'_, InstanceManagerState>,
    running: tauri::State<'_, RunningInstances>,
    auth_state: tauri::State<'_, AuthState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // Check if already running
    {
        let guard = running.lock().await;
        if guard.contains_key(&instance_id) {
            return Err("Instance is already running".into());
        }
    }

    // Get instance config from DB
    crate::instances::ensure_manager(&mgr_state, &app).await?;
    let config = {
        let guard = mgr_state.lock().await;
        let mgr = guard.as_ref().unwrap();
        mgr.get(instance_id)
            .await
            .map_err(|e| format!("failed to get instance: {e}"))?
            .ok_or_else(|| format!("instance {instance_id} not found"))?
    };

    let instance_path = PathBuf::from(&config.instance_path);
    let mc_version = config.minecraft_version.clone();
    let loader = config.loader.clone();
    let loader_version = config.loader_version.clone();
    let instance_name = config.name.clone();

    // Get auth credentials
    let (username, uuid, access_token) = {
        let auth = auth_state.lock().map_err(|e| format!("auth lock: {e}"))?;
        match &auth.session {
            Some(UserSession::Microsoft { username, uuid, .. }) => {
                let token = auth.access_token.clone().unwrap_or_else(|| "0".into());
                (username.clone(), uuid.clone(), token)
            }
            Some(UserSession::Offline { username, uuid }) => {
                (username.clone(), uuid.clone(), "0".into())
            }
            Some(UserSession::Demo { username, uuid }) => {
                (username.clone(), uuid.clone(), "0".into())
            }
            None => ("Player".into(), "00000000-0000-0000-0000-000000000000".into(), "0".into()),
        }
    };

    // Ensure game files are downloaded (also resolves java version + main class from manifest)
    let game = ensure_game_files(&app, instance_id, &instance_name, &mc_version, &instance_path).await?;

    // Ensure Java is available — use the component from the MC manifest
    let java_path = ensure_java(&app, instance_id, &instance_name, &game.java_component, game.java_major).await?;

    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir: {e}"))?;
    let assets_dir = data_dir.join("assets");

    // Read per-instance settings for JVM args and memory
    let settings_path = instance_path.join("lodestone_settings.json");
    let (mem_mb, jvm_args_str) = if settings_path.exists() {
        let data = std::fs::read_to_string(&settings_path).unwrap_or_default();
        let settings: serde_json::Value = serde_json::from_str(&data).unwrap_or_default();
        let mem = settings.get("maxMemoryMb").and_then(|v| v.as_u64()).map(|v| v as u32);
        let args = settings.get("jvmArguments").and_then(|v| v.as_str()).map(|s| s.to_string());
        (mem, args)
    } else {
        (None, None)
    };

    let mem = mem_mb.unwrap_or(4096);
    let default_jvm = format!("-Xmx{mem}M -Xms512M");
    let jvm_str = jvm_args_str.unwrap_or(default_jvm);
    let jvm_args: Vec<&str> = jvm_str.split_whitespace().collect();

    // Marker file to track whether the loader has been installed for this version combo
    let loader_marker = instance_path.join(".lodestone-loader-installed");

    let command = match loader {
        LoaderType::Vanilla => {
            build_vanilla_command(
                &java_path,
                &instance_path,
                &game.client_jar,
                &mc_version,
                &game.main_class,
                &assets_dir,
                &game.asset_index,
                &jvm_args,
                &username,
                &uuid,
                &access_token,
            )?
        }
        LoaderType::Fabric => {
            let fabric = FabricModLoader::new();
            let lv = loader_version
                .as_deref()
                .ok_or("Fabric loader version not set")?;

            if !loader_marker.exists() {
                emit_progress(&app, &InstallProgress {
                    instance_id,
                    instance_name: instance_name.clone(),
                    stage: "loader".into(),
                    stage_label: "Installing Fabric...".into(),
                    progress: 0.5,
                    files_done: 0,
                    files_total: 0,
                });
                fabric
                    .install_client(&mc_version, lv, &instance_path, &game.client_jar, &java_path)
                    .await
                    .map_err(|e| format!("Fabric install failed: {e}"))?;
                let _ = std::fs::write(&loader_marker, format!("fabric-{lv}-{mc_version}"));
            }

            fabric
                .run_fabric_client(
                    &instance_path,
                    &game.client_jar,
                    lv,
                    &mc_version,
                    &jvm_args,
                    &java_path,
                    &assets_dir,
                    &game.asset_index,
                    &instance_path,
                )
                .map_err(|e| format!("Fabric launch failed: {e}"))?
        }
        LoaderType::Forge => {
            let forge = ForgeModLoader::new();
            let lv = loader_version
                .as_deref()
                .ok_or("Forge loader version not set")?;

            if !loader_marker.exists() {
                emit_progress(&app, &InstallProgress {
                    instance_id,
                    instance_name: instance_name.clone(),
                    stage: "loader".into(),
                    stage_label: "Installing Forge...".into(),
                    progress: 0.5,
                    files_done: 0,
                    files_total: 0,
                });
                log::info!("Installing Forge {lv} for MC {mc_version}");
                forge
                    .install_client(&mc_version, lv, &instance_path, &game.client_jar, &java_path)
                    .await
                    .map_err(|e| format!("Forge install failed: {e}"))?;
                let _ = std::fs::write(&loader_marker, format!("forge-{lv}-{mc_version}"));
            }

            forge
                .run_forge_client(
                    &instance_path,
                    &game.client_jar,
                    &mc_version,
                    lv,
                    &jvm_args,
                    &java_path,
                    &assets_dir,
                    &game.asset_index,
                    &instance_path,
                )
                .map_err(|e| format!("Forge launch failed: {e}"))?
        }
        LoaderType::Neoforge => {
            let forge = ForgeModLoader::new();
            let lv = loader_version
                .as_deref()
                .ok_or("NeoForge loader version not set")?;

            if !loader_marker.exists() {
                emit_progress(&app, &InstallProgress {
                    instance_id,
                    instance_name: instance_name.clone(),
                    stage: "loader".into(),
                    stage_label: "Installing NeoForge...".into(),
                    progress: 0.5,
                    files_done: 0,
                    files_total: 0,
                });
                log::info!("Installing NeoForge {lv} for MC {mc_version}");
                // NeoForge uses its own Maven repository
                let installer_url = minecraft_modloaders::neoforge::NeoForgeVersions::installer_url(lv);
                log::info!("NeoForge installer URL: {installer_url}");
                forge
                    .install_client_from_url(&installer_url, &mc_version, lv, &instance_path, &game.client_jar, &java_path)
                    .await
                    .map_err(|e| format!("NeoForge install failed: {e}"))?;
                let _ = std::fs::write(&loader_marker, format!("neoforge-{lv}-{mc_version}"));
            }

            forge
                .run_forge_client(
                    &instance_path,
                    &game.client_jar,
                    &mc_version,
                    lv,
                    &jvm_args,
                    &java_path,
                    &assets_dir,
                    &game.asset_index,
                    &instance_path,
                )
                .map_err(|e| format!("NeoForge launch failed: {e}"))?
        }
        LoaderType::Quilt => {
            let fabric = FabricModLoader::new();
            let lv = loader_version
                .as_deref()
                .ok_or("Quilt loader version not set")?;

            if !loader_marker.exists() {
                emit_progress(&app, &InstallProgress {
                    instance_id,
                    instance_name: instance_name.clone(),
                    stage: "loader".into(),
                    stage_label: "Installing Quilt...".into(),
                    progress: 0.5,
                    files_done: 0,
                    files_total: 0,
                });
                fabric
                    .install_client(&mc_version, lv, &instance_path, &game.client_jar, &java_path)
                    .await
                    .map_err(|e| format!("Quilt install failed: {e}"))?;
                let _ = std::fs::write(&loader_marker, format!("quilt-{lv}-{mc_version}"));
            }

            fabric
                .run_fabric_client(
                    &instance_path,
                    &game.client_jar,
                    lv,
                    &mc_version,
                    &jvm_args,
                    &java_path,
                    &assets_dir,
                    &game.asset_index,
                    &instance_path,
                )
                .map_err(|e| format!("Quilt launch failed: {e}"))?
        }
    };

    // Spawn the game process
    let child = tokio::process::Command::from(command)
        .spawn()
        .map_err(|e| format!("failed to spawn game: {e}"))?;

    // Store in running instances
    {
        let mut guard = running.lock().await;
        guard.insert(instance_id, child);
    }

    let _ = app.emit("instance-started", instance_id);

    // Update last_played
    {
        let guard = mgr_state.lock().await;
        let mgr = guard.as_ref().unwrap();
        let _ = mgr.touch(instance_id).await;
    }

    // Spawn background monitor for process exit
    let running_clone = running.inner().clone();
    let app_clone = app.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            let mut guard = running_clone.lock().await;
            if let Some(child) = guard.get_mut(&instance_id) {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        guard.remove(&instance_id);
                        let _ = app_clone.emit("instance-stopped", instance_id);
                        break;
                    }
                    Ok(None) => {}
                    Err(_) => {
                        guard.remove(&instance_id);
                        let _ = app_clone.emit("instance-stopped", instance_id);
                        break;
                    }
                }
            } else {
                break;
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_instance(
    instance_id: i64,
    running: tauri::State<'_, RunningInstances>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let mut guard = running.lock().await;
    if let Some(mut child) = guard.remove(&instance_id) {
        child
            .kill()
            .await
            .map_err(|e| format!("failed to kill process: {e}"))?;
        let _ = app.emit("instance-stopped", instance_id);
        Ok(())
    } else {
        Err("Instance is not running".into())
    }
}

#[tauri::command]
pub async fn get_running_instances(
    running: tauri::State<'_, RunningInstances>,
) -> Result<Vec<i64>, String> {
    let guard = running.lock().await;
    Ok(guard.keys().copied().collect())
}
