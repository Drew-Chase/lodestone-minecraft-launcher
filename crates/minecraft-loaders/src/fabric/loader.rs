use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::ModLoader;

const API_URL: &str = "https://meta.fabricmc.net/v2/versions/";
const SERVER_LAUNCH_JAR_URL: &str = "https://meta.fabricmc.net/v2/versions/loader";

/// Response from the Fabric meta API containing all version information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricVersions {
    pub game: Vec<GameVersion>,
    pub loader: Vec<LoaderVersion>,
    pub intermediary: Vec<IntermediaryVersion>,
    pub installer: Vec<InstallerVersion>,
}

/// A Minecraft game version supported by Fabric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameVersion {
    pub version: String,
    pub stable: bool,
}

/// A Fabric loader version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderVersion {
    pub separator: String,
    pub build: u32,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

/// An intermediary mappings version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntermediaryVersion {
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

/// A Fabric installer version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallerVersion {
    pub url: String,
    pub maven: String,
    pub version: String,
    pub stable: bool,
}

impl FabricVersions {
    /// Fetches all available Fabric versions from the meta API.
    pub async fn fetch() -> Result<Self> {
        let response = reqwest::get(API_URL).await?;
        let versions = response.json::<Self>().await?;
        Ok(versions)
    }

    /// Gets the latest stable installer version.
    pub fn get_latest_installer(&self) -> Option<&InstallerVersion> {
        self.installer
            .iter()
            .filter(|v| v.stable)
            .max_by(|a, b| a.version.cmp(&b.version))
    }

    /// Gets the latest stable loader version.
    pub fn get_latest_loader(&self) -> Option<&LoaderVersion> {
        self.loader
            .iter()
            .filter(|v| v.stable)
            .max_by(|a, b| a.version.cmp(&b.version))
    }

    /// Gets the latest stable game version.
    pub fn get_latest_game_version(&self) -> Option<&GameVersion> {
        self.game.iter().find(|v| v.stable)
    }

    /// Finds a specific loader version by version string.
    pub fn find_loader(&self, version: &str) -> Option<&LoaderVersion> {
        self.loader.iter().find(|v| v.version == version)
    }

    /// Finds a specific installer version by version string.
    pub fn find_installer(&self, version: &str) -> Option<&InstallerVersion> {
        self.installer.iter().find(|v| v.version == version)
    }

    /// Finds a specific game version by version string.
    pub fn find_game_version(&self, version: &str) -> Option<&GameVersion> {
        self.game.iter().find(|v| v.version == version)
    }
}

impl InstallerVersion {
    /// Downloads the installer JAR to a specified path.
    pub async fn download(&self, output_path: impl AsRef<Path>) -> Result<PathBuf> {
        let response = reqwest::get(&self.url).await?;
        let bytes = response.bytes().await?;
        let output = output_path.as_ref();

        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent).await?;
        }

        let mut file = fs::File::create(output).await?;
        file.write_all(&bytes).await?;

        Ok(output.to_path_buf())
    }
}


/// Fabric mod loader implementation.
///
/// # Example
///
/// ```rust,no_run
/// use minecraft_modloaders::fabric::FabricModLoader;
/// use minecraft_modloaders::ModLoader;
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let loader = FabricModLoader::new();
///
///     // Download a server JAR
///     loader.download_server(
///         "1.20.1",
///         "0.14.24",
///         Path::new("./server/fabric-server.jar"),
///     ).await?;
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct FabricModLoader;

impl FabricModLoader {
    /// Creates a new FabricModLoader instance.
    pub fn new() -> Self {
        Self
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

    /// Creates a Command for running a JAR file with Java.
    /// All paths are canonicalized to absolute paths to avoid issues when the working directory is changed.
    fn create_java_command(
        working_dir: &Path,
        jar_path: &Path,
        arguments: &[&str],
        java_path: &Path,
    ) -> Result<std::process::Command> {
        // Canonicalize paths to absolute paths to avoid issues with working directory changes
        let abs_java_path = dunce::canonicalize(java_path)
            .with_context(|| format!("Failed to canonicalize java path: {}", java_path.display()))?;
        let abs_jar_path = dunce::canonicalize(jar_path)
            .with_context(|| format!("Failed to canonicalize jar path: {}", jar_path.display()))?;
        let abs_working_dir = dunce::canonicalize(working_dir)
            .with_context(|| format!("Failed to canonicalize working directory: {}", working_dir.display()))?;

        let mut command = std::process::Command::new(&abs_java_path);
        command.current_dir(&abs_working_dir);

        // Add user arguments before -jar
        for arg in arguments {
            command.arg(arg);
        }

        command.arg("-jar").arg(&abs_jar_path);
        Ok(command)
    }

    /// Recursively collects all JAR files from a directory.
    fn collect_jars_recursive(dir: &Path, jars: &mut Vec<String>) -> Result<()> {
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)
                .with_context(|| format!("Failed to read directory: {}", dir.display()))?
            {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    Self::collect_jars_recursive(&path, jars)?;
                } else if path.extension().is_some_and(|ext| ext == "jar") {
                    jars.push(path.display().to_string());
                }
            }
        }
        Ok(())
    }

    /// Creates a Command for running the Fabric client with proper classpath and main class.
    ///
    /// This method loads the Fabric version JSON and builds the correct classpath
    /// from all required libraries, using the main class specified in the version JSON.
    ///
    /// # Arguments
    /// * `install_dir` - The installation directory (where versions/ and libraries/ are located)
    /// * `client_jar_path` - Path to the Minecraft client JAR
    /// * `loader_version` - The Fabric loader version (e.g., "0.14.24")
    /// * `mc_version` - The Minecraft version (e.g., "1.20.1")
    /// * `jvm_arguments` - Additional JVM arguments (e.g., "-Xmx2G")
    /// * `java_path` - Path to the Java executable
    /// * `assets_dir` - Path to the assets directory (where assets are downloaded)
    /// * `asset_index` - The asset index name (usually the MC version or a specific index like "17")
    /// * `game_dir` - The game directory (for saves, configs, etc.)
    #[allow(clippy::too_many_arguments)]
    pub fn run_fabric_client(
        &self,
        install_dir: &Path,
        client_jar_path: &Path,
        loader_version: &str,
        mc_version: &str,
        jvm_arguments: &[&str],
        java_path: &Path,
        assets_dir: &Path,
        asset_index: &str,
        game_dir: &Path,
    ) -> Result<std::process::Command> {
        use crate::fabric::version_json::VersionJson;

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

        // Load the version JSON
        let version_json = VersionJson::load(&abs_install_dir, loader_version, mc_version)
            .with_context(|| format!(
                "Failed to load Fabric version JSON for loader {} and MC {}",
                loader_version, mc_version
            ))?;

        // Build classpath from ALL JARs in the libraries directory
        // This includes both Fabric libraries and Minecraft libraries
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

        let mut command = std::process::Command::new(&abs_java_path);
        command.current_dir(&abs_install_dir);

        // Add user JVM arguments first
        for arg in jvm_arguments {
            command.arg(arg);
        }

        // Add JVM arguments from the version JSON
        for jvm_arg in &version_json.arguments.jvm {
            command.arg(jvm_arg);
        }

        // Add classpath
        command.arg("-cp").arg(&classpath);

        // Add main class
        command.arg(&version_json.main_class);

        // Add game arguments from the version JSON
        for game_arg in &version_json.arguments.game {
            command.arg(game_arg);
        }

        // Add required Minecraft game arguments for assets and directories
        command
            .arg("--version").arg(format!("fabric-loader-{}-{}", loader_version, mc_version))
            .arg("--gameDir").arg(&abs_game_dir)
            .arg("--assetsDir").arg(&abs_assets_dir)
            .arg("--assetIndex").arg(asset_index)
            // Offline mode credentials (required but can be dummy values)
            .arg("--accessToken").arg("0")
            .arg("--username").arg("Player")
            .arg("--uuid").arg("00000000-0000-0000-0000-000000000000");

        Ok(command)
    }
}

#[async_trait]
impl ModLoader for FabricModLoader {
    async fn install_server(
        &self,
        minecraft_version: &str,
        loader_version: &str,
        server_path: &Path,
        java_path: &Path,
    ) -> Result<PathBuf> {
        let versions = FabricVersions::fetch().await?;
        let installer = versions
            .get_latest_installer()
            .ok_or_else(|| anyhow!("No installer version available"))?;

        // Create server directory and download installer
        fs::create_dir_all(server_path).await?;
        let installer_path = server_path.join(format!("fabric-installer-{}.jar", &installer.version));
        installer.download(&installer_path).await?;

        // Canonicalize paths for the installer
        let abs_installer_path = dunce::canonicalize(&installer_path)
            .with_context(|| format!("Failed to canonicalize installer path: {}", installer_path.display()))?;
        let abs_java_path = dunce::canonicalize(java_path)
            .with_context(|| format!("Failed to canonicalize java path: {}", java_path.display()))?;
        let abs_server_path = dunce::canonicalize(server_path)
            .with_context(|| format!("Failed to canonicalize server path: {}", server_path.display()))?;

        // Run installer
        let output = tokio::process::Command::new(&abs_java_path)
            .arg("-jar")
            .arg(&abs_installer_path)
            .arg("server")
            .arg("-dir")
            .arg(&abs_server_path)
            .arg("-mcversion")
            .arg(minecraft_version)
            .arg("-loader")
            .arg(loader_version)
            .arg("-downloadMinecraft")
            .output()
            .await
            .context("Failed to execute Fabric installer")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Fabric installer failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(server_path.join("fabric-server-launch.jar"))
    }

    async fn download_server(
        &self,
        minecraft_version: &str,
        loader_version: &str,
        file_path: &Path,
    ) -> Result<PathBuf> {
        let url = format!(
            "{}/{}/{}/server/jar",
            SERVER_LAUNCH_JAR_URL, minecraft_version, loader_version
        );
        Self::download_file(&url, file_path).await
    }

    fn run_server(
        &self,
        working_dir: &Path,
        server_jar_path: &Path,
        arguments: &[&str],
        java_path: &Path,
    ) -> Result<std::process::Command> {
        Self::create_java_command(
            working_dir,
            server_jar_path,
            arguments,
            java_path,
        )
    }

    async fn install_client(
        &self,
        minecraft_version: &str,
        loader_version: &str,
        install_dir: &Path,
        client_path: &Path,
        java_path: &Path,
    ) -> Result<PathBuf> {
        let versions = FabricVersions::fetch().await?;
        let installer = versions
            .get_latest_installer()
            .ok_or_else(|| anyhow!("No installer version available"))?;

        // Create install directory and download installer
        fs::create_dir_all(install_dir).await?;
        let installer_path =
            install_dir.join(format!("fabric-installer-{}.jar", &installer.version));
        installer.download(&installer_path).await?;

        // Canonicalize paths for the installer
        let abs_installer_path = dunce::canonicalize(&installer_path)
            .with_context(|| format!("Failed to canonicalize installer path: {}", installer_path.display()))?;
        let abs_java_path = dunce::canonicalize(java_path)
            .with_context(|| format!("Failed to canonicalize java path: {}", java_path.display()))?;
        let abs_install_dir = dunce::canonicalize(install_dir)
            .with_context(|| format!("Failed to canonicalize install directory: {}", install_dir.display()))?;

        // Run installer
        let output = tokio::process::Command::new(&abs_java_path)
            .arg("-jar")
            .arg(&abs_installer_path)
            .arg("client")
            .arg("-dir")
            .arg(&abs_install_dir)
            .arg("-mcversion")
            .arg(minecraft_version)
            .arg("-loader")
            .arg(loader_version)
            .arg("-noprofile")
            .output()
            .await
            .context("Failed to execute Fabric installer")?;

        if !output.status.success() {
            return Err(anyhow!(
                "Fabric installer failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(client_path.to_path_buf())
    }

    async fn download_client(
        &self,
        _minecraft_version: &str,
        _loader_version: &str,
        file_path: &Path,
    ) -> Result<PathBuf> {
        // Fabric client requires the installer - download it to the specified path
        let versions = FabricVersions::fetch().await?;
        let installer = versions
            .get_latest_installer()
            .ok_or_else(|| anyhow!("No installer version available"))?;

        installer.download(file_path).await
    }

    fn run_client(
        &self,
        working_dir: &Path,
        client_jar_path: &Path,
        arguments: &[&str],
        java_path: &Path,
    ) -> Result<std::process::Command> {
        Self::create_java_command(
            working_dir,
            client_jar_path,
            arguments,
            java_path,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_versions() {
        let versions = FabricVersions::fetch().await.unwrap();
        assert!(!versions.game.is_empty());
        assert!(!versions.loader.is_empty());
        assert!(!versions.installer.is_empty());
        assert!(!versions.intermediary.is_empty());
    }

    #[tokio::test]
    async fn test_get_latest_versions() {
        let versions = FabricVersions::fetch().await.unwrap();

        let latest_installer = versions.get_latest_installer();
        assert!(latest_installer.is_some());
        assert!(latest_installer.unwrap().stable);

        let latest_loader = versions.get_latest_loader();
        assert!(latest_loader.is_some());
        assert!(latest_loader.unwrap().stable);

        let latest_game = versions.get_latest_game_version();
        assert!(latest_game.is_some());
        assert!(latest_game.unwrap().stable);
    }

    #[tokio::test]
    async fn test_find_specific_versions() {
        let versions = FabricVersions::fetch().await.unwrap();

        if let Some(first_loader) = versions.loader.first() {
            let found = versions.find_loader(&first_loader.version);
            assert!(found.is_some());
            assert_eq!(found.unwrap().version, first_loader.version);
        }

        if let Some(first_game) = versions.game.first() {
            let found = versions.find_game_version(&first_game.version);
            assert!(found.is_some());
            assert_eq!(found.unwrap().version, first_game.version);
        }

        if let Some(first_installer) = versions.installer.first() {
            let found = versions.find_installer(&first_installer.version);
            assert!(found.is_some());
            assert_eq!(found.unwrap().version, first_installer.version);
        }
    }

    #[tokio::test]
    async fn test_loader_version_properties() {
        let versions = FabricVersions::fetch().await.unwrap();

        if let Some(loader) = versions.loader.first() {
            assert!(!loader.version.is_empty());
            assert!(!loader.maven.is_empty());
            assert!(loader.build > 0);
        }
    }

    #[tokio::test]
    async fn test_installer_version_properties() {
        let versions = FabricVersions::fetch().await.unwrap();

        if let Some(installer) = versions.installer.first() {
            assert!(!installer.version.is_empty());
            assert!(!installer.maven.is_empty());
            assert!(!installer.url.is_empty());
            assert!(installer.url.starts_with("http"));
        }
    }

    #[tokio::test]
    async fn test_game_version_properties() {
        let versions = FabricVersions::fetch().await.unwrap();

        if let Some(game) = versions.game.first() {
            assert!(!game.version.is_empty());
        }
    }

    #[tokio::test]
    async fn test_download_installer() {
        let versions = FabricVersions::fetch().await.unwrap();
        let installer = versions
            .get_latest_installer()
            .expect("No installer found");

        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join(format!(
            "test-fabric-installer-{}.jar",
            installer.version
        ));

        let result = installer.download(&output_path).await;
        assert!(result.is_ok());

        let downloaded_path = result.unwrap();
        assert!(downloaded_path.exists());

        // Cleanup
        let _ = std::fs::remove_file(downloaded_path);
    }

    #[test]
    fn test_version_ordering() {
        let versions = [
            LoaderVersion {
                separator: ".".to_string(),
                build: 1,
                maven: "test1".to_string(),
                version: "0.14.0".to_string(),
                stable: false,
            },
            LoaderVersion {
                separator: ".".to_string(),
                build: 2,
                maven: "test2".to_string(),
                version: "0.15.0".to_string(),
                stable: true,
            },
        ];

        let latest_stable = versions
            .iter()
            .filter(|v| v.stable)
            .max_by(|a, b| a.version.cmp(&b.version));

        assert!(latest_stable.is_some());
        assert_eq!(latest_stable.unwrap().version, "0.15.0");
    }
}
