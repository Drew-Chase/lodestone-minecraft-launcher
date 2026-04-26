pub mod fabric;
pub mod forge;
pub mod neoforge;
pub mod quilt;

use anyhow::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ModLoaderError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parsing error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Zip extraction error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Version not found: {0}")]
    VersionNotFound(String),

    #[error("Invalid version format: {0}")]
    InvalidVersion(String),

    #[error("Installation failed: {0}")]
    InstallationFailed(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Java execution error: {0}")]
    JavaError(String),
    
    #[error("Incorrect Java version: {0} (required: {1})")]
    IncorrectJavaVersion(String, String),
}

/// Trait for mod loader implementations (Fabric, Forge, etc.)
///
/// This trait provides async methods for installing, downloading, and running
/// Minecraft mod loaders for both server and client.
///
/// # Note
/// `run_server` and `run_client` are synchronous as they only construct `Command` objects
/// without performing any I/O operations.
#[async_trait]
pub trait ModLoader: Send + Sync {
    /// Installs a Minecraft server with the specified versions and configurations.
    ///
    /// # Parameters
    /// - `minecraft_version`: The Minecraft version (e.g., "1.20.1")
    /// - `loader_version`: The mod loader version (e.g., "0.14.24")
    /// - `server_path`: Directory where the server should be installed
    /// - `java_path`: Path to the Java executable
    ///
    /// # Returns
    /// Path to the installed server JAR file
    async fn install_server(
        &self,
        minecraft_version: &str,
        loader_version: &str,
        server_path: &Path,
        java_path: &Path,
    ) -> Result<PathBuf>;

    /// Downloads a Minecraft server JAR file.
    ///
    /// # Parameters
    /// - `minecraft_version`: The Minecraft version (e.g., "1.20.1")
    /// - `loader_version`: The mod loader version (e.g., "0.14.24")
    /// - `file_path`: Where to save the downloaded JAR file
    ///
    /// # Returns
    /// Path to the downloaded file
    async fn download_server(
        &self,
        minecraft_version: &str,
        loader_version: &str,
        file_path: &Path,
    ) -> Result<PathBuf>;

    /// Creates a Command to run a Minecraft server.
    ///
    /// # Parameters
    /// - `working_dir`: Working directory for the server process
    /// - `server_jar_path`: Path to the server JAR file
    /// - `arguments`: JVM arguments (e.g., ["-Xmx4G", "-Xms2G"])
    /// - `java_path`: Path to the Java executable
    ///
    /// # Returns
    /// A configured `Command` ready to be spawned
    fn run_server(
        &self,
        working_dir: &Path,
        server_jar_path: &Path,
        arguments: &[&str],
        java_path: &Path,
    ) -> Result<std::process::Command>;

    /// Installs a Minecraft client with the specified versions and paths.
    ///
    /// # Parameters
    /// - `minecraft_version`: The Minecraft version (e.g., "1.20.1")
    /// - `loader_version`: The mod loader version (e.g., "0.14.24")
    /// - `library_directory`: Directory for libraries/instance files
    /// - `client_path`: Path where the client should be installed
    /// - `java_path`: Path to the Java executable
    ///
    /// # Returns
    /// Path to the installed client
    async fn install_client(
        &self,
        minecraft_version: &str,
        loader_version: &str,
        library_directory: &Path,
        client_path: &Path,
        java_path: &Path,
    ) -> Result<PathBuf>;

    /// Downloads the client files for a specific version.
    ///
    /// # Parameters
    /// - `minecraft_version`: The Minecraft version (e.g., "1.20.1")
    /// - `loader_version`: The mod loader version (e.g., "0.14.24")
    /// - `file_path`: Where to save the downloaded file
    ///
    /// # Returns
    /// Path to the downloaded file
    async fn download_client(
        &self,
        minecraft_version: &str,
        loader_version: &str,
        file_path: &Path,
    ) -> Result<PathBuf>;

    /// Creates a Command to run a Minecraft client.
    ///
    /// # Parameters
    /// - `working_dir`: Working directory for the client process
    /// - `client_jar_path`: Path to the client JAR file
    /// - `arguments`: JVM arguments (e.g., ["-Xmx4G", "-Xms2G"])
    /// - `java_path`: Path to the Java executable
    ///
    /// # Returns
    /// A configured `Command` ready to be spawned
    fn run_client(
        &self,
        working_dir: &Path,
        client_jar_path: &Path,
        arguments: &[&str],
        java_path: &Path,
    ) -> Result<std::process::Command>;
}
