use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::Manager;

// ---------------------------------------------------------------------------
// Settings struct
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    // General
    pub instance_dir: String,
    pub startup_behavior: String,
    pub on_game_launch: String,
    pub after_game_exits: String,
    pub auto_update: bool,
    pub beta_channel: bool,
    pub auto_update_games: bool,
    pub concurrent_downloads: u32,
    // Java
    pub max_memory_mb: u32,
    pub default_java_path: String,
    pub jvm_arguments: String,
    // Appearance
    pub accent_color: String,
    pub theme: String,
    pub animations: bool,
    pub particles: bool,
    pub glass: bool,
    pub aurora: bool,
    pub reduce_motion: bool,
    pub font: String,
    // Network
    pub max_concurrent_downloads: u32,
    pub mod_source_priority: String,
    pub asset_cdn: String,
    pub connection_timeout: u32,
    pub use_system_proxy: bool,
    pub custom_proxy_url: String,
    // Privacy
    pub crash_reports: bool,
    pub usage_stats: bool,
    pub performance_diagnostics: bool,
    pub filesystem_access: bool,
    pub network_access: bool,
    pub hardware_access: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // General
            instance_dir: String::new(),
            startup_behavior: "Show library".into(),
            on_game_launch: "Minimize launcher".into(),
            after_game_exits: "Restore launcher".into(),
            auto_update: true,
            beta_channel: false,
            auto_update_games: true,
            concurrent_downloads: 11,
            // Java
            max_memory_mb: 8192,
            default_java_path: String::new(),
            jvm_arguments: "-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200\n-XX:+UnlockExperimentalVMOptions -XX:+DisableExplicitGC -XX:+AlwaysPreTouch".into(),
            // Appearance
            accent_color: "#22ff84".into(),
            theme: "void".into(),
            animations: true,
            particles: true,
            glass: true,
            aurora: true,
            reduce_motion: false,
            font: "Inter".into(),
            // Network
            max_concurrent_downloads: 8,
            mod_source_priority: "Modrinth → CurseForge".into(),
            asset_cdn: "Auto (closest)".into(),
            connection_timeout: 30,
            use_system_proxy: true,
            custom_proxy_url: String::new(),
            // Privacy
            crash_reports: true,
            usage_stats: true,
            performance_diagnostics: false,
            filesystem_access: true,
            network_access: true,
            hardware_access: false,
        }
    }
}

// ---------------------------------------------------------------------------
// On-disk persistence
// ---------------------------------------------------------------------------

fn settings_file_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    Ok(dir.join("settings.json"))
}

fn load_settings_from_disk(app: &tauri::AppHandle) -> Result<Settings, String> {
    let path = settings_file_path(app)?;
    if !path.exists() {
        return Ok(Settings::default());
    }
    let data = std::fs::read_to_string(&path).map_err(|e| format!("failed to read settings.json: {e}"))?;
    let settings: Settings = serde_json::from_str(&data).map_err(|e| format!("failed to parse settings.json: {e}"))?;
    Ok(settings)
}

fn save_settings_to_disk(app: &tauri::AppHandle, settings: &Settings) -> Result<(), String> {
    let path = settings_file_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("failed to create app data dir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("failed to write settings.json: {e}"))
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    let mut settings = load_settings_from_disk(&app)?;
    // Fill in the real default instance dir if it was never set.
    if settings.instance_dir.is_empty()
        && let Ok(dir) = app.path().app_data_dir()
    {
        settings.instance_dir = dir.join("instances").to_string_lossy().to_string();
    }
    Ok(settings)
}

#[tauri::command]
pub async fn save_settings(settings: Settings, app: tauri::AppHandle) -> Result<(), String> {
    save_settings_to_disk(&app, &settings)
}

#[tauri::command]
pub async fn reset_settings(app: tauri::AppHandle) -> Result<Settings, String> {
    if let Ok(path) = settings_file_path(&app) {
        let _ = std::fs::remove_file(path);
    }
    Ok(Settings::default())
}

#[tauri::command]
pub async fn get_system_ram() -> Result<u64, String> {
    use sysinfo::System;
    let mut sys = System::new();
    sys.refresh_memory();
    Ok(sys.total_memory() / 1_048_576) // bytes to MB
}
