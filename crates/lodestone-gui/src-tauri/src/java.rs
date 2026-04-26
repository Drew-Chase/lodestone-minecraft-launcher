use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use tauri::{Emitter, Manager};
use tokio::process::Command;

// ---------------------------------------------------------------------------
// Types exposed to the frontend
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedJava {
    pub label: String,
    pub version: String,
    pub path: String,
    pub source: String, // "system" | "managed"
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MojangRuntime {
    pub component: String,
    pub java_version: u8,
    pub label: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct JavaInstallProgress {
    component: String,
    files_downloaded: u32,
    files_total: u32,
}

// ---------------------------------------------------------------------------
// Java version detection helpers
// ---------------------------------------------------------------------------

/// Run `java -version` and parse the version string from stderr.
async fn probe_java_version(java_exe: &Path) -> Option<String> {
    let output = tokio::time::timeout(
        Duration::from_secs(5),
        Command::new(java_exe).arg("-version").output(),
    )
    .await
    .ok()?
    .ok()?;

    // `java -version` writes to stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    parse_version_from_output(&stderr)
}

fn parse_version_from_output(output: &str) -> Option<String> {
    // Matches patterns like: "17.0.11" or "1.8.0_412"
    for line in output.lines() {
        // Look for quoted version string: "21.0.3" or "1.8.0_412"
        if let Some(start) = line.find('"')
            && let Some(end) = line[start + 1..].find('"') {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
    }
    None
}

fn major_version_from_string(version: &str) -> Option<u8> {
    let first = version.split('.').next()?;
    let major: u8 = first.parse().ok()?;
    // Java 1.x style (Java 8 = "1.8.0_xxx")
    if major == 1 {
        version.split('.').nth(1)?.parse().ok()
    } else {
        Some(major)
    }
}

fn label_from_version_and_path(version: &str, path: &Path) -> String {
    let major = major_version_from_string(version).unwrap_or(0);
    let vendor = guess_vendor_from_path(path);
    if vendor.is_empty() {
        format!("Java {major}")
    } else {
        format!("Java {major} · {vendor}")
    }
}

fn guess_vendor_from_path(path: &Path) -> &str {
    let s = path.to_string_lossy();
    let lower = s.to_ascii_lowercase();
    if lower.contains("temurin") || lower.contains("adoptium") {
        "Temurin"
    } else if lower.contains("zulu") {
        "Zulu"
    } else if lower.contains("corretto") {
        "Corretto"
    } else if lower.contains("graalvm") {
        "GraalVM"
    } else if lower.contains("microsoft") {
        "Microsoft"
    } else if lower.contains("liberica") || lower.contains("bellsoft") {
        "Liberica"
    } else if lower.contains("semeru") || lower.contains("ibm") {
        "Semeru"
    } else if lower.contains("oracle") || lower.contains("javasoft") {
        "Oracle"
    } else {
        ""
    }
}

/// Find java.exe inside a JDK/JRE directory.
fn find_java_exe(base: &Path) -> Option<PathBuf> {
    let bin_java = base.join("bin").join(java_exe_name());
    if bin_java.exists() {
        return Some(bin_java);
    }
    None
}

fn java_exe_name() -> &'static str {
    if cfg!(windows) {
        "java.exe"
    } else {
        "java"
    }
}

// ---------------------------------------------------------------------------
// System Java candidate collection
// ---------------------------------------------------------------------------

fn collect_system_java_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // 1. JAVA_HOME
    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let p = PathBuf::from(&java_home);
        if let Some(exe) = find_java_exe(&p) {
            candidates.push(exe);
        }
    }

    // 2. PATH entries
    if let Ok(path_var) = std::env::var("PATH") {
        let sep = if cfg!(windows) { ';' } else { ':' };
        for dir in path_var.split(sep) {
            let exe = PathBuf::from(dir).join(java_exe_name());
            if exe.exists() {
                candidates.push(exe);
            }
        }
    }

    // 3. Common install directories (Windows)
    #[cfg(windows)]
    {
        let program_files_dirs: Vec<PathBuf> = [
            std::env::var("ProgramFiles").ok(),
            std::env::var("ProgramFiles(x86)").ok(),
            std::env::var("ProgramW6432").ok(),
        ]
        .into_iter()
        .flatten()
        .map(PathBuf::from)
        .collect();

        let search_dirs = [
            "Java",
            "Eclipse Adoptium",
            "Microsoft",
            "Zulu",
            "Amazon Corretto",
            "BellSoft",
            "AdoptOpenJDK",
        ];

        for pf in &program_files_dirs {
            for search in &search_dirs {
                let base = pf.join(search);
                if base.is_dir()
                    && let Ok(entries) = std::fs::read_dir(&base) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.is_dir()
                                && let Some(exe) = find_java_exe(&path) {
                                    candidates.push(exe);
                                }
                        }
                    }
            }
        }
    }

    // 4. Windows Registry
    #[cfg(windows)]
    {
        candidates.extend(collect_from_registry());
    }

    candidates
}

#[cfg(windows)]
fn collect_from_registry() -> Vec<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut results = Vec::new();
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);

    let reg_paths = [
        r"SOFTWARE\JavaSoft\Java Development Kit",
        r"SOFTWARE\JavaSoft\Java Runtime Environment",
        r"SOFTWARE\JavaSoft\JDK",
        r"SOFTWARE\JavaSoft\JRE",
    ];

    for reg_path in &reg_paths {
        if let Ok(key) = hklm.open_subkey_with_flags(reg_path, KEY_READ)
            && let Ok(subkeys) = key.enum_keys().collect::<Result<Vec<_>, _>>() {
                for subkey_name in subkeys {
                    if let Ok(subkey) = key.open_subkey_with_flags(&subkey_name, KEY_READ)
                        && let Ok(java_home) = subkey.get_value::<String, _>("JavaHome") {
                            let p = PathBuf::from(&java_home);
                            if let Some(exe) = find_java_exe(&p) {
                                results.push(exe);
                            }
                        }
                }
            }
    }

    results
}

/// Collect managed (piston-mc downloaded) runtimes from the app data dir.
fn collect_managed_candidates(java_dir: &Path) -> Vec<(PathBuf, String)> {
    let mut results = Vec::new();
    if !java_dir.is_dir() {
        return results;
    }
    if let Ok(entries) = std::fs::read_dir(java_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let component = entry
                    .file_name()
                    .to_string_lossy()
                    .to_string();
                if let Some(exe) = find_java_exe(&path) {
                    results.push((exe, component));
                }
            }
        }
    }
    results
}

// ---------------------------------------------------------------------------
// Platform selection for piston-mc JavaManifest
// ---------------------------------------------------------------------------

fn select_platform_runtimes(
    manifest: &piston_mc::java::JavaManifest,
) -> &piston_mc::java::Runtimes {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        &manifest.windows_x64
    }
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        &manifest.windows_x86
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        &manifest.linux
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        &manifest.macos
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        &manifest.macos_arm64
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn detect_system_java(app: tauri::AppHandle) -> Result<Vec<DetectedJava>, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    let java_dir = data_dir.join("java");

    let mut results = Vec::new();
    let mut seen_paths: HashSet<PathBuf> = HashSet::new();

    // System-installed Java
    let candidates = collect_system_java_candidates();
    let mut handles = Vec::new();
    for candidate in candidates {
        let canonical = match candidate.canonicalize() {
            Ok(c) => c,
            Err(_) => candidate.clone(),
        };
        if !seen_paths.insert(canonical) {
            continue;
        }
        let c = candidate.clone();
        handles.push(tokio::spawn(async move {
            let version = probe_java_version(&c).await;
            (c, version)
        }));
    }

    for handle in handles {
        if let Ok((path, Some(version))) = handle.await {
            let label = label_from_version_and_path(&version, &path);
            // Get the JDK root (parent of bin/)
            let root = path
                .parent()
                .and_then(|p| p.parent())
                .unwrap_or(&path);
            results.push(DetectedJava {
                label,
                version,
                path: root.to_string_lossy().to_string(),
                source: "system".into(),
            });
        }
    }

    // Managed runtimes (downloaded via piston-mc)
    let managed = collect_managed_candidates(&java_dir);
    for (exe, component) in managed {
        let canonical = match exe.canonicalize() {
            Ok(c) => c,
            Err(_) => exe.clone(),
        };
        if !seen_paths.insert(canonical) {
            continue;
        }
        if let Some(version) = probe_java_version(&exe).await {
            let label = format!(
                "Java {} · Mojang ({})",
                major_version_from_string(&version).unwrap_or(0),
                component
            );
            let root = exe.parent().and_then(|p| p.parent()).unwrap_or(&exe);
            results.push(DetectedJava {
                label,
                version,
                path: root.to_string_lossy().to_string(),
                source: "managed".into(),
            });
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn get_available_java_runtimes(
    app: tauri::AppHandle,
) -> Result<Vec<MojangRuntime>, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    let java_dir = data_dir.join("java");

    let manifest = piston_mc::java::JavaManifest::fetch()
        .await
        .map_err(|e| format!("failed to fetch Java manifest: {e}"))?;

    let platform = select_platform_runtimes(&manifest);

    let components: Vec<(&str, u8, &str)> = vec![
        ("delta", 21, "Java 21"),
        ("gamma", 17, "Java 17"),
        ("alpha", 16, "Java 16"),
        ("legacy", 8, "Java 8"),
    ];

    let mut runtimes = Vec::new();
    for (component, version, label) in components {
        let has_runtime = match component {
            "delta" => !platform.delta.is_empty(),
            "gamma" => !platform.gamma.is_empty(),
            "alpha" => !platform.alpha.is_empty(),
            "legacy" => !platform.legacy.is_empty(),
            _ => false,
        };

        if has_runtime {
            let install_dir = java_dir.join(component);
            let java_exe = install_dir.join("bin").join(java_exe_name());
            runtimes.push(MojangRuntime {
                component: component.to_string(),
                java_version: version,
                label: label.to_string(),
                installed: java_exe.exists(),
            });
        }
    }

    Ok(runtimes)
}

#[tauri::command]
pub async fn install_java_runtime(
    component: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    let java_dir = data_dir.join("java");
    let install_dir = java_dir.join(&component);

    tokio::fs::create_dir_all(&install_dir)
        .await
        .map_err(|e: std::io::Error| format!("failed to create java install dir: {e}"))?;

    let manifest = piston_mc::java::JavaManifest::fetch()
        .await
        .map_err(|e| format!("failed to fetch Java manifest: {e}"))?;

    let platform = select_platform_runtimes(&manifest);

    let runtime = match component.as_str() {
        "delta" => platform.delta.first(),
        "gamma" => platform.gamma.first(),
        "alpha" => platform.alpha.first(),
        "legacy" => platform.legacy.first(),
        _ => return Err(format!("unknown Java component: {component}")),
    }
    .ok_or_else(|| format!("no runtime found for component: {component}"))?;

    let (sender, mut receiver) =
        tokio::sync::mpsc::channel::<piston_mc::download_util::MultiDownloadProgress>(16);
    let component_clone = component.clone();
    let app_clone = app.clone();

    // Spawn task to forward progress events to the frontend
    tokio::spawn(async move {
        while let Some(message) = receiver.recv().await {
            let _ = app_clone.emit(
                "java-install-progress",
                JavaInstallProgress {
                    component: component_clone.clone(),
                    files_downloaded: message.files_downloaded as u32,
                    files_total: message.files_total as u32,
                },
            );
        }
    });

    let result: Result<(), _> = runtime
        .install(&install_dir, 20, Some(sender))
        .await;
    result.map_err(|e| format!("failed to install Java runtime: {e}"))?;

    let java_exe = install_dir.join("bin").join(java_exe_name());
    Ok(java_exe.to_string_lossy().to_string())
}
