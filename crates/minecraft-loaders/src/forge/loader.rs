use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::ModLoader;

const VERSIONS_URL: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/maven-metadata.json";
const MAVEN_BASE_URL: &str = "https://maven.minecraftforge.net/net/minecraftforge/forge";

/// Forge version era determines how the server/client should be launched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeEra {
    /// Minecraft ≤1.16.5: Simple universal JAR, run with `java -jar`
    Legacy,
    /// Minecraft 1.17 - 1.20.2: Uses @args.txt for complex JVM arguments
    ArgsFile,
    /// Minecraft ≥1.20.3: Shim JAR approach, back to simple `java -jar`
    Modern,
}

impl ForgeEra {
    /// Determine the Forge era from a Minecraft version string.
    pub fn from_minecraft_version(version: &str) -> Self {
        // Parse major.minor from version string
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() < 2 {
            return ForgeEra::Legacy;
        }

        let major: u32 = parts[0].parse().unwrap_or(1);
        let minor: u32 = parts[1].parse().unwrap_or(0);
        let patch: u32 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

        match (major, minor, patch) {
            (1, minor, _) if minor <= 16 => ForgeEra::Legacy,
            (1, minor, patch) if minor == 20 && patch >= 3 => ForgeEra::Modern,
            (1, minor, _) if minor >= 21 => ForgeEra::Modern,
            (1, 17..=20, _) => ForgeEra::ArgsFile,
            _ => ForgeEra::Legacy,
        }
    }
}

/// Forge versions data from the Forge API.
/// Contains all available Forge versions for each Minecraft version.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ForgeVersions {
    /// Map of Minecraft version -> list of full version strings (e.g., "1.20.1-47.2.0")
    pub versions: HashMap<String, Vec<String>>,
}

impl ForgeVersions {
    /// Fetch all available Forge versions from the API.
    pub async fn fetch() -> Result<Self> {
        let response = reqwest::get(VERSIONS_URL)
            .await
            .context("Failed to fetch Forge versions")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch Forge versions: HTTP {}",
                response.status()
            ));
        }

        let versions: HashMap<String, Vec<String>> = response
            .json()
            .await
            .context("Failed to parse Forge versions")?;

        Ok(Self { versions })
    }

    /// Get all Forge versions for a Minecraft version.
    /// Returns version strings like "47.2.0" (the Forge version part only).
    pub fn get_versions(&self, minecraft_version: &str) -> Option<Vec<&str>> {
        self.versions.get(minecraft_version).map(|versions| {
            versions
                .iter()
                .filter_map(|v| {
                    // Parse "1.20.1-47.2.0" to get "47.2.0"
                    v.split_once('-').map(|(_, forge)| forge)
                })
                .collect()
        })
    }

    /// Get the latest Forge version for a Minecraft version.
    pub fn get_latest(&self, minecraft_version: &str) -> Option<&str> {
        self.versions.get(minecraft_version).and_then(|versions| {
            versions.last().and_then(|v| v.split_once('-').map(|(_, forge)| forge))
        })
    }

    /// Get all supported Minecraft versions.
    pub fn get_supported_minecraft_versions(&self) -> Vec<&str> {
        let mut versions: Vec<&str> = self.versions.keys().map(|s| s.as_str()).collect();
        // Sort by version (simple string sort works for most cases)
        versions.sort_by(|a, b| {
            let parse = |v: &str| -> (u32, u32, u32) {
                let parts: Vec<u32> = v.split('.').filter_map(|p| p.parse().ok()).collect();
                (
                    parts.first().copied().unwrap_or(0),
                    parts.get(1).copied().unwrap_or(0),
                    parts.get(2).copied().unwrap_or(0),
                )
            };
            parse(b).cmp(&parse(a)) // Reverse order (newest first)
        });
        versions
    }
}

/// Forge mod loader implementation.
///
/// Handles three different eras of Forge:
/// - Legacy (≤1.16.5): Simple universal JAR
/// - Args Era (1.17 - 1.20.2): Uses @args.txt files
/// - Modern (≥1.20.3): Shim JAR approach
///
/// # Example
///
/// ```rust,no_run
/// use minecraft_modloaders::forge::ForgeModLoader;
/// use minecraft_modloaders::ModLoader;
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let loader = ForgeModLoader::new();
///
///     // Install a Forge server
///     loader.install_server(
///         "1.20.1",
///         "47.2.0",
///         Path::new("./server"),
///         Path::new("/usr/bin/java"),
///     ).await?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct ForgeModLoader;

impl ForgeModLoader {
    /// Creates a new ForgeModLoader instance.
    pub fn new() -> Self {
        Self
    }

    /// Get the installer JAR URL for a specific Minecraft and Forge version.
    fn get_installer_url(minecraft_version: &str, forge_version: &str) -> String {
        format!(
            "{}/{}-{}/forge-{}-{}-installer.jar",
            MAVEN_BASE_URL, minecraft_version, forge_version, minecraft_version, forge_version
        )
    }

    /// Install a Forge-compatible client from a custom installer URL.
    /// Used by NeoForge which has a different Maven repository but the
    /// same installer JAR format.
    pub async fn install_client_from_url(
        &self,
        installer_url: &str,
        minecraft_version: &str,
        loader_version: &str,
        library_directory: &Path,
        client_path: &Path,
        _java_path: &Path,
    ) -> Result<PathBuf> {
        let era = ForgeEra::from_minecraft_version(minecraft_version);
        fs::create_dir_all(library_directory).await?;

        let installer_path = library_directory.join(format!(
            "loader-{}-{}-installer.jar",
            minecraft_version, loader_version
        ));

        Self::download_file(installer_url, &installer_path).await?;

        Self::extract_and_install_client(
            &installer_path,
            library_directory,
            minecraft_version,
            loader_version,
            era,
        ).await?;

        let _ = fs::remove_file(&installer_path).await;
        Ok(client_path.to_path_buf())
    }

    /// Downloads a file from a URL to the specified path.
    async fn download_file(url: &str, output_path: &Path) -> Result<PathBuf> {
        let response = reqwest::get(url)
            .await
            .context("Failed to download file")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download file: HTTP {}",
                response.status()
            ));
        }

        let bytes = response.bytes().await?;

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = fs::File::create(output_path).await?;
        file.write_all(&bytes).await?;

        Ok(output_path.to_path_buf())
    }

    /// Find the server JAR after installation based on the Forge era.
    fn find_server_jar(
        server_path: &Path,
        minecraft_version: &str,
        forge_version: &str,
        era: ForgeEra,
    ) -> PathBuf {
        match era {
            ForgeEra::Legacy => {
                // Try different legacy naming conventions
                let universal = server_path.join(format!(
                    "forge-{}-{}-universal.jar",
                    minecraft_version, forge_version
                ));
                if universal.exists() {
                    return universal;
                }
                // Fallback to standard forge jar
                server_path.join(format!(
                    "forge-{}-{}.jar",
                    minecraft_version, forge_version
                ))
            }
            ForgeEra::ArgsFile => {
                // For args era, return the run script location or a marker
                // The actual execution uses @args.txt
                server_path.join(format!(
                    "libraries/net/minecraftforge/forge/{}-{}/forge-{}-{}-server.jar",
                    minecraft_version, forge_version, minecraft_version, forge_version
                ))
            }
            ForgeEra::Modern => {
                // Try shim jar first, then server.jar
                let shim = server_path.join(format!(
                    "forge-{}-{}-shim.jar",
                    minecraft_version, forge_version
                ));
                if shim.exists() {
                    return shim;
                }
                // Modern forge often creates a simple server.jar
                let server_jar = server_path.join("server.jar");
                if server_jar.exists() {
                    return server_jar;
                }
                // Fallback
                server_path.join(format!(
                    "forge-{}-{}.jar",
                    minecraft_version, forge_version
                ))
            }
        }
    }

    /// Find the args file for ArgsFile era Forge.
    fn find_args_file(server_path: &Path, minecraft_version: &str, forge_version: &str) -> Option<PathBuf> {
        // Check for unix_args.txt (cross-platform compatible)
        let unix_args = server_path.join(format!(
            "libraries/net/minecraftforge/forge/{}-{}/unix_args.txt",
            minecraft_version, forge_version
        ));
        if unix_args.exists() {
            return Some(unix_args);
        }

        // Check for win_args.txt as fallback
        let win_args = server_path.join(format!(
            "libraries/net/minecraftforge/forge/{}-{}/win_args.txt",
            minecraft_version, forge_version
        ));
        if win_args.exists() {
            return Some(win_args);
        }

        None
    }

    /// Extract and install Forge client from installer JAR.
    /// This method extracts necessary files from the installer JAR and downloads required libraries.
    async fn extract_and_install_client(
        installer_path: &Path,
        library_directory: &Path,
        minecraft_version: &str,
        forge_version: &str,
        era: ForgeEra,
    ) -> Result<()> {
        // Open the installer JAR as a ZIP file
        let file = std::fs::File::open(installer_path)
            .context("Failed to open installer JAR")?;
        let mut archive = zip::ZipArchive::new(file)
            .context("Failed to read installer JAR as ZIP")?;

        // Extract version.json (contains library information)
        let version_json = Self::extract_json_from_archive(&mut archive, "version.json")
            .or_else(|_| Self::extract_json_from_archive(&mut archive, "install_profile.json"))?;

        // Parse libraries from the version JSON
        if let Some(libraries) = version_json.get("libraries").and_then(|v| v.as_array()) {
            Self::download_forge_libraries(libraries, library_directory).await?;
        }

        // For modern and args eras, also check install_profile.json
        if (era == ForgeEra::Modern || era == ForgeEra::ArgsFile)
            && let Ok(install_profile) = Self::extract_json_from_archive(&mut archive, "install_profile.json")
            && let Some(libraries) = install_profile.get("libraries").and_then(|v| v.as_array())
        {
            Self::download_forge_libraries(libraries, library_directory).await?;
        }

        // Extract forge universal jar (for legacy) or forge jar (for modern/args)
        Self::extract_forge_jars(&mut archive, library_directory, minecraft_version, forge_version, era).await?;

        Ok(())
    }

    /// Extract a JSON file from the ZIP archive
    fn extract_json_from_archive<R: Read + Seek>(
        archive: &mut zip::ZipArchive<R>,
        filename: &str,
    ) -> Result<serde_json::Value> {
        let mut file = archive
            .by_name(filename)
            .with_context(|| format!("Failed to find {} in installer", filename))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .with_context(|| format!("Failed to read {} from installer", filename))?;

        serde_json::from_str(&contents)
            .with_context(|| format!("Failed to parse {} as JSON", filename))
    }

    /// Download Forge libraries from the libraries array
    async fn download_forge_libraries(
        libraries: &[serde_json::Value],
        library_directory: &Path,
    ) -> Result<()> {
        for lib in libraries {
            // Parse library name (format: "group:artifact:version" or "group:artifact:version:classifier")
            if let Some(name) = lib.get("name").and_then(|v| v.as_str()) {
                let parts: Vec<&str> = name.split(':').collect();
                if parts.len() >= 3 {
                    let group = parts[0].replace('.', "/");
                    let artifact = parts[1];
                    let version = parts[2];
                    let classifier = parts.get(3).copied();

                    // Get the path from downloads if available, otherwise construct it
                    let (url, lib_path) = if let Some(downloads) = lib.get("downloads") {
                        if let Some(artifact_info) = downloads.get("artifact") {
                            let url = artifact_info
                                .get("url")
                                .and_then(|u| u.as_str())
                                .filter(|s| !s.is_empty())
                                .map(|s| s.to_string());

                            let path = artifact_info
                                .get("path")
                                .and_then(|p| p.as_str())
                                .map(|p| library_directory.join(p));

                            (url, path)
                        } else {
                            (None, None)
                        }
                    } else {
                        (None, None)
                    };

                    // If no URL from downloads, try Maven repositories
                    let url = url.or_else(|| {
                        let jar_name = if let Some(c) = classifier {
                            format!("{}-{}-{}.jar", artifact, version, c)
                        } else {
                            format!("{}-{}.jar", artifact, version)
                        };

                        // Try Forge Maven first, then Minecraft libraries
                        Some(format!(
                            "https://maven.minecraftforge.net/{}/{}/{}/{}",
                            group, artifact, version, jar_name
                        ))
                    });

                    // If no path from downloads, construct it
                    let lib_path = lib_path.unwrap_or_else(|| {
                        let jar_name = if let Some(c) = classifier {
                            format!("{}-{}-{}.jar", artifact, version, c)
                        } else {
                            format!("{}-{}.jar", artifact, version)
                        };
                        library_directory
                            .join(&group)
                            .join(artifact)
                            .join(version)
                            .join(jar_name)
                    });

                    // Download if doesn't exist and we have a URL
                    if let Some(url_str) = url
                        && !lib_path.exists()
                    {
                        // Try primary URL first
                        let result = Self::download_file(&url_str, &lib_path).await;

                        // If failed, try Minecraft libraries as fallback
                        if result.is_err() {
                            let jar_name = if let Some(c) = classifier {
                                format!("{}-{}-{}.jar", artifact, version, c)
                            } else {
                                format!("{}-{}.jar", artifact, version)
                            };
                            let mc_url = format!(
                                "https://libraries.minecraft.net/{}/{}/{}/{}",
                                group, artifact, version, jar_name
                            );
                            let _ = Self::download_file(&mc_url, &lib_path).await;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Extract Forge JAR files from the installer's maven directory
    async fn extract_forge_jars<R: Read + Seek>(
        archive: &mut zip::ZipArchive<R>,
        library_directory: &Path,
        _minecraft_version: &str,
        _forge_version: &str,
        _era: ForgeEra,
    ) -> Result<()> {
        // Extract all files from the maven/ directory in the installer
        let maven_prefix = "maven/";

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();

            if name.starts_with(maven_prefix) && name.ends_with(".jar") {
                // Remove "maven/" prefix to get the relative path
                let relative_path = &name[maven_prefix.len()..];
                let output_path = library_directory.join(relative_path);

                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent).await?;
                }

                let mut contents = Vec::new();
                file.read_to_end(&mut contents)?;
                fs::write(&output_path, contents).await?;
            }
        }

        Ok(())
    }

    /// Create the Java command for running a Forge server/client.
    /// All paths are canonicalized to absolute paths to avoid issues when the working directory is changed.
    fn create_forge_command(
        working_dir: &Path,
        jar_or_args_path: &Path,
        user_arguments: &[&str],
        java_path: &Path,
        era: ForgeEra,
        minecraft_version: &str,
        forge_version: &str,
    ) -> Result<std::process::Command> {
        // Canonicalize paths to absolute paths to avoid issues with working directory changes
        let abs_java_path = dunce::canonicalize(java_path)
            .with_context(|| format!("Failed to canonicalize java path: {}", java_path.display()))?;
        let abs_working_dir = dunce::canonicalize(working_dir)
            .with_context(|| format!("Failed to canonicalize working directory: {}", working_dir.display()))?;
        let abs_jar_or_args_path = dunce::canonicalize(jar_or_args_path)
            .with_context(|| format!("Failed to canonicalize jar/args path: {}", jar_or_args_path.display()))?;

        let mut command = std::process::Command::new(&abs_java_path);
        command.current_dir(&abs_working_dir);

        match era {
            ForgeEra::Legacy | ForgeEra::Modern => {
                // Simple jar execution
                for arg in user_arguments {
                    command.arg(arg);
                }
                command.arg("-jar").arg(&abs_jar_or_args_path);
            }
            ForgeEra::ArgsFile => {
                // Use @args.txt approach
                if let Some(args_file) = Self::find_args_file(working_dir, minecraft_version, forge_version) {
                    // Canonicalize the args file path
                    let abs_args_file = dunce::canonicalize(&args_file)
                        .with_context(|| format!("Failed to canonicalize args file: {}", args_file.display()))?;
                    // The args file contains classpath and main class
                    command.arg(format!("@{}", abs_args_file.display()));
                    for arg in user_arguments {
                        command.arg(arg);
                    }
                } else {
                    // Fallback if args file not found - try direct jar
                    for arg in user_arguments {
                        command.arg(arg);
                    }
                    command.arg("-jar").arg(&abs_jar_or_args_path);
                }
            }
        }

        Ok(command)
    }
}

#[async_trait]
impl ModLoader for ForgeModLoader {
    async fn install_server(
        &self,
        minecraft_version: &str,
        loader_version: &str,
        server_path: &Path,
        java_path: &Path,
    ) -> Result<PathBuf> {
        let era = ForgeEra::from_minecraft_version(minecraft_version);

        // Create server directory
        fs::create_dir_all(server_path).await?;

        // Download installer
        let installer_url = Self::get_installer_url(minecraft_version, loader_version);
        let installer_path = server_path.join(format!(
            "forge-{}-{}-installer.jar",
            minecraft_version, loader_version
        ));

        Self::download_file(&installer_url, &installer_path).await?;

        // Canonicalize paths for the installer
        let abs_installer_path = dunce::canonicalize(&installer_path)
            .with_context(|| format!("Failed to canonicalize installer path: {}", installer_path.display()))?;
        let abs_java_path = dunce::canonicalize(java_path)
            .with_context(|| format!("Failed to canonicalize java path: {}", java_path.display()))?;
        let abs_server_path = dunce::canonicalize(server_path)
            .with_context(|| format!("Failed to canonicalize server path: {}", server_path.display()))?;

        // Run installer with --installServer
        let output = tokio::process::Command::new(&abs_java_path)
            .current_dir(&abs_server_path)
            .arg("-jar")
            .arg(&abs_installer_path)
            .arg("--installServer")
            .output()
            .await
            .context("Failed to execute Forge installer")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Forge installer failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Clean up installer
        let _ = fs::remove_file(&installer_path).await;

        // Return path to server JAR
        Ok(Self::find_server_jar(server_path, minecraft_version, loader_version, era))
    }

    async fn download_server(
        &self,
        minecraft_version: &str,
        loader_version: &str,
        file_path: &Path,
    ) -> Result<PathBuf> {
        // Forge doesn't have pre-built server JARs, download the installer instead
        let installer_url = Self::get_installer_url(minecraft_version, loader_version);
        Self::download_file(&installer_url, file_path).await
    }

    fn run_server(
        &self,
        working_dir: &Path,
        server_jar_path: &Path,
        arguments: &[&str],
        java_path: &Path,
    ) -> Result<std::process::Command> {
        // Try to detect the Minecraft version from the jar path
        let jar_name = server_jar_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Parse version from jar name (e.g., "forge-1.20.1-47.2.0.jar")
        let (minecraft_version, forge_version) = Self::parse_forge_jar_name(jar_name)
            .unwrap_or(("1.20.4", "unknown")); // Default to modern era

        let era = ForgeEra::from_minecraft_version(minecraft_version);

        Self::create_forge_command(
            working_dir,
            server_jar_path,
            arguments,
            java_path,
            era,
            minecraft_version,
            forge_version,
        )
    }

    async fn install_client(
        &self,
        minecraft_version: &str,
        loader_version: &str,
        library_directory: &Path,
        client_path: &Path,
        _java_path: &Path,
    ) -> Result<PathBuf> {
        let era = ForgeEra::from_minecraft_version(minecraft_version);

        // Create library directory
        fs::create_dir_all(library_directory).await?;

        // Download installer
        let installer_url = Self::get_installer_url(minecraft_version, loader_version);
        let installer_path = library_directory.join(format!(
            "forge-{}-{}-installer.jar",
            minecraft_version, loader_version
        ));

        Self::download_file(&installer_url, &installer_path).await?;

        // Extract and process the installer JAR instead of running it
        // This avoids GUI issues and works across all Forge eras
        Self::extract_and_install_client(
            &installer_path,
            library_directory,
            minecraft_version,
            loader_version,
            era,
        ).await?;

        // Clean up installer
        let _ = fs::remove_file(&installer_path).await;

        Ok(client_path.to_path_buf())
    }

    async fn download_client(
        &self,
        minecraft_version: &str,
        loader_version: &str,
        file_path: &Path,
    ) -> Result<PathBuf> {
        // Download the installer JAR
        let installer_url = Self::get_installer_url(minecraft_version, loader_version);
        Self::download_file(&installer_url, file_path).await
    }

    fn run_client(
        &self,
        working_dir: &Path,
        client_jar_path: &Path,
        arguments: &[&str],
        java_path: &Path,
    ) -> Result<std::process::Command> {
        // Similar to run_server but for client
        let jar_name = client_jar_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        let (minecraft_version, forge_version) = Self::parse_forge_jar_name(jar_name)
            .unwrap_or(("1.20.4", "unknown"));

        let era = ForgeEra::from_minecraft_version(minecraft_version);

        Self::create_forge_command(
            working_dir,
            client_jar_path,
            arguments,
            java_path,
            era,
            minecraft_version,
            forge_version,
        )
    }
}

impl ForgeModLoader {
    /// Run a Forge client with proper classpath and arguments.
    /// This is the recommended way to run a Forge client, as it properly sets up
    /// the classpath, main class, and game arguments.
    ///
    /// # Arguments
    ///
    /// * `install_dir` - The installation directory containing libraries
    /// * `client_jar_path` - Path to the Minecraft client JAR
    /// * `minecraft_version` - The Minecraft version (e.g., "1.12.2")
    /// * `forge_version` - The Forge version (e.g., "14.23.5.2864")
    /// * `jvm_arguments` - Additional JVM arguments (e.g., "-Xmx2G")
    /// * `java_path` - Path to the Java executable
    /// * `assets_dir` - Path to the assets directory
    /// * `asset_index` - The asset index name
    /// * `game_dir` - The game directory (for saves, configs, etc.)
    #[allow(clippy::too_many_arguments)]
    pub fn run_forge_client(
        &self,
        install_dir: &Path,
        client_jar_path: &Path,
        minecraft_version: &str,
        _forge_version: &str,
        jvm_arguments: &[&str],
        java_path: &Path,
        assets_dir: &Path,
        asset_index: &str,
        game_dir: &Path,
    ) -> Result<std::process::Command> {
        let era = ForgeEra::from_minecraft_version(minecraft_version);

        // Canonicalize paths
        let abs_java_path = dunce::canonicalize(java_path)
            .with_context(|| format!("Failed to canonicalize java path: {}", java_path.display()))?;
        let abs_install_dir = dunce::canonicalize(install_dir)
            .with_context(|| format!("Failed to canonicalize install directory: {}", install_dir.display()))?;
        let abs_client_jar = dunce::canonicalize(client_jar_path)
            .with_context(|| format!("Failed to canonicalize client jar path: {}", client_jar_path.display()))?;
        let abs_assets_dir = dunce::canonicalize(assets_dir)
            .with_context(|| format!("Failed to canonicalize assets directory: {}", assets_dir.display()))?;
        let abs_game_dir = dunce::canonicalize(game_dir)
            .with_context(|| format!("Failed to canonicalize game directory: {}", game_dir.display()))?;

        // Build classpath from ALL JARs in the libraries directory
        let libraries_dir = abs_install_dir.join("libraries");
        let mut classpath_entries: Vec<String> = Vec::new();

        if libraries_dir.exists() {
            Self::collect_jars_recursive(&libraries_dir, &mut classpath_entries)?;
        }

        // Add the client JAR to the classpath
        classpath_entries.push(abs_client_jar.display().to_string());

        // Build the classpath string (semicolon on Windows, colon on Unix)
        let classpath_separator = if cfg!(windows) { ";" } else { ":" };
        let classpath = classpath_entries.join(classpath_separator);

        // Extract natives and set up natives directory
        let natives_dir = abs_install_dir.join("natives");
        if !natives_dir.exists() {
            std::fs::create_dir_all(&natives_dir)?;
        }
        Self::extract_natives(&libraries_dir, &natives_dir)?;

        let mut command = std::process::Command::new(&abs_java_path);
        command.current_dir(&abs_install_dir);

        // Add native library path
        command.arg(format!("-Djava.library.path={}", natives_dir.display()));

        // For modern Forge (Java 9+), add module opens
        if matches!(era, ForgeEra::ArgsFile | ForgeEra::Modern) {
            command
                .arg("--add-opens").arg("java.base/java.util.jar=ALL-UNNAMED")
                .arg("--add-opens").arg("java.base/java.lang.invoke=ALL-UNNAMED")
                .arg("--add-opens").arg("java.base/java.lang=ALL-UNNAMED")
                .arg("--add-opens").arg("java.base/java.util=ALL-UNNAMED")
                .arg("--add-modules").arg("ALL-MODULE-PATH");
        }

        // Add user JVM arguments
        for arg in jvm_arguments {
            command.arg(arg);
        }

        // Add classpath
        command.arg("-cp").arg(&classpath);

        // Determine main class based on era
        let main_class = match era {
            ForgeEra::Legacy => "net.minecraft.launchwrapper.Launch",
            ForgeEra::ArgsFile | ForgeEra::Modern => "cpw.mods.bootstraplauncher.BootstrapLauncher",
        };
        command.arg(main_class);

        // Add Forge-specific arguments
        match era {
            ForgeEra::Legacy => {
                // Legacy Forge uses tweaker classes
                command
                    .arg("--tweakClass").arg("net.minecraftforge.fml.common.launcher.FMLTweaker")
                    .arg("--version").arg(minecraft_version)
                    .arg("--gameDir").arg(&abs_game_dir)
                    .arg("--assetsDir").arg(&abs_assets_dir)
                    .arg("--assetIndex").arg(asset_index);
            }
            ForgeEra::ArgsFile | ForgeEra::Modern => {
                // Modern Forge uses different approach
                command
                    .arg("--version").arg(minecraft_version)
                    .arg("--gameDir").arg(&abs_game_dir)
                    .arg("--assetsDir").arg(&abs_assets_dir)
                    .arg("--assetIndex").arg(asset_index);
            }
        }

        Ok(command)
    }

    /// Extract native libraries from JAR files to the natives directory
    fn extract_natives(libraries_dir: &Path, natives_dir: &Path) -> Result<()> {
        // Find all native JAR files
        let mut native_jars = Vec::new();
        Self::find_native_jars(libraries_dir, &mut native_jars)?;

        for jar_path in native_jars {
            let file = std::fs::File::open(&jar_path)?;
            let mut archive = zip::ZipArchive::new(file)?;

            for i in 0..archive.len() {
                let mut file = archive.by_index(i)?;
                let name = file.name().to_string();

                // Extract .dll, .so, .dylib files (native libraries)
                if name.ends_with(".dll") || name.ends_with(".so") || name.ends_with(".dylib") {
                    // Get just the filename, not the full path
                    let filename = std::path::Path::new(&name)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&name);

                    let output_path = natives_dir.join(filename);

                    // Skip if already extracted
                    if output_path.exists() {
                        continue;
                    }

                    let mut contents = Vec::new();
                    file.read_to_end(&mut contents)?;
                    std::fs::write(&output_path, contents)?;
                }
            }
        }

        Ok(())
    }

    /// Find all native JAR files in the libraries directory
    fn find_native_jars(dir: &Path, jars: &mut Vec<PathBuf>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                Self::find_native_jars(&path, jars)?;
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.contains("natives") && name.ends_with(".jar")
            {
                jars.push(path);
            }
        }

        Ok(())
    }

    /// Recursively collect all JAR files in a directory
    fn collect_jars_recursive(dir: &Path, jars: &mut Vec<String>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        let entries = std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read directory: {}", dir.display()))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                Self::collect_jars_recursive(&path, jars)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("jar") {
                jars.push(path.display().to_string());
            }
        }

        Ok(())
    }

    /// Parse a Forge JAR filename to extract Minecraft and Forge versions.
    /// Example: "forge-1.20.1-47.2.0.jar" -> Some(("1.20.1", "47.2.0"))
    fn parse_forge_jar_name(name: &str) -> Option<(&str, &str)> {
        let name = name.strip_suffix(".jar").unwrap_or(name);
        let name = name.strip_prefix("forge-").unwrap_or(name);

        // Handle different naming patterns
        // Pattern 1: "1.20.1-47.2.0" or "1.20.1-47.2.0-universal"
        let name = name.strip_suffix("-universal").unwrap_or(name);
        let name = name.strip_suffix("-shim").unwrap_or(name);
        let name = name.strip_suffix("-server").unwrap_or(name);

        // Find the split point between MC version and Forge version
        // MC versions are like "1.20.1" and Forge versions are like "47.2.0"
        let parts: Vec<&str> = name.splitn(2, '-').collect();
        if parts.len() == 2 {
            Some((parts[0], parts[1]))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forge_era_detection() {
        assert_eq!(ForgeEra::from_minecraft_version("1.12.2"), ForgeEra::Legacy);
        assert_eq!(ForgeEra::from_minecraft_version("1.16.5"), ForgeEra::Legacy);
        assert_eq!(ForgeEra::from_minecraft_version("1.17"), ForgeEra::ArgsFile);
        assert_eq!(ForgeEra::from_minecraft_version("1.17.1"), ForgeEra::ArgsFile);
        assert_eq!(ForgeEra::from_minecraft_version("1.18.2"), ForgeEra::ArgsFile);
        assert_eq!(ForgeEra::from_minecraft_version("1.19.4"), ForgeEra::ArgsFile);
        assert_eq!(ForgeEra::from_minecraft_version("1.20.1"), ForgeEra::ArgsFile);
        assert_eq!(ForgeEra::from_minecraft_version("1.20.2"), ForgeEra::ArgsFile);
        assert_eq!(ForgeEra::from_minecraft_version("1.20.3"), ForgeEra::Modern);
        assert_eq!(ForgeEra::from_minecraft_version("1.20.4"), ForgeEra::Modern);
        assert_eq!(ForgeEra::from_minecraft_version("1.21"), ForgeEra::Modern);
        assert_eq!(ForgeEra::from_minecraft_version("1.21.1"), ForgeEra::Modern);
    }

    #[test]
    fn test_parse_forge_jar_name() {
        assert_eq!(
            ForgeModLoader::parse_forge_jar_name("forge-1.20.1-47.2.0.jar"),
            Some(("1.20.1", "47.2.0"))
        );
        assert_eq!(
            ForgeModLoader::parse_forge_jar_name("forge-1.12.2-14.23.5.2860-universal.jar"),
            Some(("1.12.2", "14.23.5.2860"))
        );
        assert_eq!(
            ForgeModLoader::parse_forge_jar_name("forge-1.20.4-49.0.0-shim.jar"),
            Some(("1.20.4", "49.0.0"))
        );
    }

    #[test]
    fn test_installer_url() {
        let url = ForgeModLoader::get_installer_url("1.20.1", "47.2.0");
        assert_eq!(
            url,
            "https://maven.minecraftforge.net/net/minecraftforge/forge/1.20.1-47.2.0/forge-1.20.1-47.2.0-installer.jar"
        );
    }

    #[tokio::test]
    async fn test_fetch_versions() {
        let versions = ForgeVersions::fetch().await;
        assert!(versions.is_ok());

        let versions = versions.unwrap();
        assert!(!versions.versions.is_empty());

        // Check that we can get supported MC versions
        let mc_versions = versions.get_supported_minecraft_versions();
        assert!(!mc_versions.is_empty());
    }

    #[tokio::test]
    async fn test_get_forge_versions() {
        let versions = ForgeVersions::fetch().await.unwrap();

        // 1.20.1 should have versions available
        let forge_versions = versions.get_versions("1.20.1");
        assert!(forge_versions.is_some());
        assert!(!forge_versions.unwrap().is_empty());

        // Latest should return something
        let latest = versions.get_latest("1.20.1");
        assert!(latest.is_some());
    }
}
