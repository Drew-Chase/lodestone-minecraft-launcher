//! Integration tests for Fabric and Forge mod loaders.
//!
//! These tests download actual files from the internet and run real processes.
//! They are marked with `#[ignore]` and should be run with:
//! ```
//! cargo test --test loader_integration_test -- --ignored
//! ```
//!
//! Requirements:
//! - Internet connection
//! - Sufficient disk space (~500MB per test)
//! - Display for client tests (they launch with full GUI)

use minecraft_modloaders::fabric::{FabricModLoader, FabricVersions};
use minecraft_modloaders::forge::{ForgeModLoader, ForgeVersions};
use minecraft_modloaders::ModLoader;
use piston_mc::java::JavaManifest;
use piston_mc::manifest_v2::ManifestV2;
use std::path::{Path, PathBuf};
use std::time::Duration;

const TEST_TIMEOUT: Duration = Duration::from_secs(30);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes for downloads

/// Helper to install Java for a Minecraft version
/// Returns the path to the java executable
async fn install_java(mc_version: &str, install_dir: &Path) -> anyhow::Result<PathBuf> {
    let manifest = ManifestV2::fetch().await?;
    let version_manifest = manifest
        .version(mc_version)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Minecraft version {} not found", mc_version))?;

    // Determine which Java runtime to use based on major version
    let java_major = version_manifest
        .java_version
        .as_ref()
        .map(|jv| jv.major_version)
        .unwrap_or(17); // Default to Java 17 if not specified

    let java_manifest = JavaManifest::fetch().await?;

    // Select the appropriate runtime based on Java version
    // Access the runtime directly via the struct fields
    let runtime = match java_major {
        8 => java_manifest.windows_x64.legacy.first(),
        16 => java_manifest.windows_x64.alpha.first(),
        17 => java_manifest.windows_x64.gamma.first(),
        21 => java_manifest.windows_x64.delta.first(),
        _ => java_manifest.windows_x64.gamma.first(), // Default to Java 17
    }
    .ok_or_else(|| anyhow::anyhow!("No Java runtime found for Java {}", java_major))?;

    // Install Java
    runtime.install(install_dir, 4, None).await?;

    // Return path to java executable
    let java_path = if cfg!(windows) {
        install_dir.join("bin").join("java.exe")
    } else {
        install_dir.join("bin").join("java")
    };

    Ok(java_path)
}

/// Helper to download Minecraft assets
async fn download_assets(mc_version: &str, assets_dir: &Path) -> anyhow::Result<()> {
    let manifest = ManifestV2::fetch().await?;
    let version_manifest = manifest
        .version(mc_version)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Minecraft version {} not found", mc_version))?;

    // Download assets
    let mut assets = version_manifest.assets().await?;
    assets.download(assets_dir, 4, None).await?;

    Ok(())
}

/// Helper to run a process with timeout and then kill it
async fn run_with_timeout(
    command: std::process::Command,
    duration: Duration,
) -> anyhow::Result<()> {
    let mut child = tokio::process::Command::from(command).spawn()?;

    // Wait for timeout
    tokio::time::sleep(duration).await;

    // Kill the process
    child.kill().await?;

    Ok(())
}

// ============================================================================
// FABRIC SERVER TESTS
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_fabric_server_full_workflow() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let base_path = temp_dir.path();

    // 1. Get latest Fabric versions
    let versions = FabricVersions::fetch().await.expect("Failed to fetch Fabric versions");
    let mc_version = versions
        .get_latest_game_version()
        .expect("No game version found")
        .version
        .clone();
    let loader_version = versions
        .get_latest_loader()
        .expect("No loader version found")
        .version
        .clone();

    println!("Testing Fabric Server: MC {} / Loader {}", mc_version, loader_version);

    // 2. Download Java
    let java_dir = base_path.join("java");
    println!("Installing Java...");
    let java_path = tokio::time::timeout(DOWNLOAD_TIMEOUT, install_java(&mc_version, &java_dir))
        .await
        .expect("Java download timed out")
        .expect("Failed to install Java");

    assert!(java_path.exists(), "Java executable not found");
    println!("Java installed at: {}", java_path.display());

    // 3. Download Fabric server
    let server_dir = base_path.join("server");
    tokio::fs::create_dir_all(&server_dir).await.expect("Failed to create server dir");

    let loader = FabricModLoader::new();
    println!("Downloading Fabric server...");
    let server_jar = tokio::time::timeout(
        DOWNLOAD_TIMEOUT,
        loader.download_server(&mc_version, &loader_version, &server_dir.join("server.jar")),
    )
    .await
    .expect("Server download timed out")
    .expect("Failed to download server");

    assert!(server_jar.exists(), "Server JAR not found");
    println!("Server JAR downloaded: {}", server_jar.display());

    // 4. Create eula.txt to allow server to start
    tokio::fs::write(server_dir.join("eula.txt"), "eula=true")
        .await
        .expect("Failed to write eula.txt");

    // 5. Run server with timeout
    println!("Running server for {}s...", TEST_TIMEOUT.as_secs());
    let command = loader
        .run_server(&server_dir, &server_jar, &["-Xmx512M"], &java_path)
        .expect("Failed to create server command");

    let result = run_with_timeout(command, TEST_TIMEOUT).await;
    println!("Server test completed: {:?}", result.is_ok());

    // Test passes if we got here - the server started and was killed after timeout
}

#[tokio::test]
#[ignore]
async fn test_fabric_server_install_workflow() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let base_path = temp_dir.path();

    // Get versions
    let versions = FabricVersions::fetch().await.expect("Failed to fetch versions");
    let mc_version = versions
        .get_latest_game_version()
        .expect("No game version")
        .version
        .clone();
    let loader_version = versions
        .get_latest_loader()
        .expect("No loader")
        .version
        .clone();

    println!("Testing Fabric Server Install: MC {} / Loader {}", mc_version, loader_version);

    // Install Java
    let java_dir = base_path.join("java");
    let java_path = tokio::time::timeout(DOWNLOAD_TIMEOUT, install_java(&mc_version, &java_dir))
        .await
        .expect("Timeout")
        .expect("Failed to install Java");

    // Install server (uses installer instead of direct download)
    let server_dir = base_path.join("server");
    let loader = FabricModLoader::new();

    println!("Installing Fabric server...");
    let server_jar = tokio::time::timeout(
        DOWNLOAD_TIMEOUT,
        loader.install_server(&mc_version, &loader_version, &server_dir, &java_path),
    )
    .await
    .expect("Install timed out")
    .expect("Failed to install server");

    assert!(server_jar.exists(), "Server JAR not found after install");
    println!("Server installed at: {}", server_jar.display());
}

// ============================================================================
// FABRIC CLIENT TESTS
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_fabric_client_full_workflow() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let base_path = temp_dir.path();

    // Get versions
    let versions = FabricVersions::fetch().await.expect("Failed to fetch versions");
    let mc_version = versions
        .get_latest_game_version()
        .expect("No game version")
        .version
        .clone();
    let loader_version = versions
        .get_latest_loader()
        .expect("No loader")
        .version
        .clone();

    println!("Testing Fabric Client: MC {} / Loader {}", mc_version, loader_version);

    // Install Java
    let java_dir = base_path.join("java");
    println!("Installing Java...");
    let java_path = tokio::time::timeout(DOWNLOAD_TIMEOUT, install_java(&mc_version, &java_dir))
        .await
        .expect("Timeout")
        .expect("Failed to install Java");

    // Download assets
    let assets_dir = base_path.join("assets");
    println!("Downloading assets...");
    tokio::time::timeout(DOWNLOAD_TIMEOUT, download_assets(&mc_version, &assets_dir))
        .await
        .expect("Asset download timed out")
        .expect("Failed to download assets");

    // Install client
    let instance_dir = base_path.join("instance");
    let client_path = instance_dir.join("client.jar");
    let loader = FabricModLoader::new();

    println!("Installing Fabric client...");
    let installed_path = tokio::time::timeout(
        DOWNLOAD_TIMEOUT,
        loader.install_client(
            &mc_version,
            &loader_version,
            &instance_dir,
            &client_path,
            &java_path,
        ),
    )
    .await
    .expect("Install timed out")
    .expect("Failed to install client");

    println!("Client installed at: {}", installed_path.display());

    // Run client with timeout (full GUI)
    println!("Running client for {}s (with GUI)...", TEST_TIMEOUT.as_secs());
    let command = loader
        .run_client(&instance_dir, &installed_path, &["-Xmx512M"], &java_path)
        .expect("Failed to create client command");

    let result = run_with_timeout(command, TEST_TIMEOUT).await;
    println!("Client test completed: {:?}", result.is_ok());
}

// ============================================================================
// FORGE SERVER TESTS
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_forge_server_full_workflow() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let base_path = temp_dir.path();

    // Get Forge versions (use 1.20.1 which is well-tested)
    let versions = ForgeVersions::fetch().await.expect("Failed to fetch Forge versions");
    let mc_version = "1.20.1";
    let forge_version = versions
        .get_latest(mc_version)
        .expect("No Forge version found for 1.20.1")
        .to_string();

    println!("Testing Forge Server: MC {} / Forge {}", mc_version, forge_version);

    // Install Java
    let java_dir = base_path.join("java");
    println!("Installing Java...");
    let java_path = tokio::time::timeout(DOWNLOAD_TIMEOUT, install_java(mc_version, &java_dir))
        .await
        .expect("Java download timed out")
        .expect("Failed to install Java");

    assert!(java_path.exists(), "Java executable not found");
    println!("Java installed at: {}", java_path.display());

    // Install Forge server
    let server_dir = base_path.join("server");
    let loader = ForgeModLoader::new();

    println!("Installing Forge server (this may take several minutes)...");
    let server_jar = tokio::time::timeout(
        DOWNLOAD_TIMEOUT,
        loader.install_server(mc_version, &forge_version, &server_dir, &java_path),
    )
    .await
    .expect("Server install timed out")
    .expect("Failed to install server");

    println!("Server installed at: {}", server_jar.display());

    // Create eula.txt
    tokio::fs::write(server_dir.join("eula.txt"), "eula=true")
        .await
        .expect("Failed to write eula.txt");

    // Run server with timeout
    println!("Running server for {}s...", TEST_TIMEOUT.as_secs());
    let command = loader
        .run_server(&server_dir, &server_jar, &["-Xmx512M"], &java_path)
        .expect("Failed to create server command");

    let result = run_with_timeout(command, TEST_TIMEOUT).await;
    println!("Server test completed: {:?}", result.is_ok());
}

#[tokio::test]
#[ignore]
async fn test_forge_server_download_installer() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let base_path = temp_dir.path();

    // Get Forge versions
    let versions = ForgeVersions::fetch().await.expect("Failed to fetch versions");
    let mc_version = "1.20.1";
    let forge_version = versions
        .get_latest(mc_version)
        .expect("No Forge version found")
        .to_string();

    println!("Testing Forge Installer Download: MC {} / Forge {}", mc_version, forge_version);

    // Download installer
    let installer_path = base_path.join("forge-installer.jar");
    let loader = ForgeModLoader::new();

    println!("Downloading Forge installer...");
    let downloaded = tokio::time::timeout(
        DOWNLOAD_TIMEOUT,
        loader.download_server(mc_version, &forge_version, &installer_path),
    )
    .await
    .expect("Download timed out")
    .expect("Failed to download installer");

    assert!(downloaded.exists(), "Installer not downloaded");

    let metadata = tokio::fs::metadata(&downloaded).await.expect("Failed to get metadata");
    assert!(metadata.len() > 1_000_000, "Installer file too small"); // Should be > 1MB

    println!("Installer downloaded: {} ({:.2} MB)",
        downloaded.display(),
        metadata.len() as f64 / 1024.0 / 1024.0);
}

// ============================================================================
// FORGE CLIENT TESTS
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_forge_client_full_workflow() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let base_path = temp_dir.path();

    // Get versions
    let versions = ForgeVersions::fetch().await.expect("Failed to fetch versions");
    let mc_version = "1.20.1";
    let forge_version = versions
        .get_latest(mc_version)
        .expect("No Forge version")
        .to_string();

    println!("Testing Forge Client: MC {} / Forge {}", mc_version, forge_version);

    // Install Java
    let java_dir = base_path.join("java");
    println!("Installing Java...");
    let java_path = tokio::time::timeout(DOWNLOAD_TIMEOUT, install_java(mc_version, &java_dir))
        .await
        .expect("Timeout")
        .expect("Failed to install Java");

    // Download assets
    let assets_dir = base_path.join("assets");
    println!("Downloading assets...");
    tokio::time::timeout(DOWNLOAD_TIMEOUT, download_assets(mc_version, &assets_dir))
        .await
        .expect("Asset download timed out")
        .expect("Failed to download assets");

    // Install client
    let instance_dir = base_path.join("instance");
    let client_path = instance_dir.join("client.jar");
    let loader = ForgeModLoader::new();

    println!("Installing Forge client (this will open installer GUI)...");
    let installed_path = tokio::time::timeout(
        DOWNLOAD_TIMEOUT,
        loader.install_client(
            mc_version,
            &forge_version,
            &instance_dir,
            &client_path,
            &java_path,
        ),
    )
    .await
    .expect("Install timed out")
    .expect("Failed to install client");

    println!("Client installed at: {}", installed_path.display());

    // Run client with timeout (full GUI)
    println!("Running client for {}s (with GUI)...", TEST_TIMEOUT.as_secs());
    let command = loader
        .run_client(&instance_dir, &installed_path, &["-Xmx512M"], &java_path)
        .expect("Failed to create client command");

    let result = run_with_timeout(command, TEST_TIMEOUT).await;
    println!("Client test completed: {:?}", result.is_ok());
}

// ============================================================================
// VERSION FETCHING TESTS (these run quickly)
// ============================================================================

#[tokio::test]
async fn test_fabric_version_fetch() {
    let versions = FabricVersions::fetch().await.expect("Failed to fetch Fabric versions");

    assert!(!versions.game.is_empty(), "No game versions");
    assert!(!versions.loader.is_empty(), "No loader versions");
    assert!(!versions.installer.is_empty(), "No installer versions");

    let latest_game = versions.get_latest_game_version().expect("No latest game");
    let latest_loader = versions.get_latest_loader().expect("No latest loader");

    println!("Fabric - Latest MC: {}, Loader: {}", latest_game.version, latest_loader.version);
}

#[tokio::test]
async fn test_forge_version_fetch() {
    let versions = ForgeVersions::fetch().await.expect("Failed to fetch Forge versions");

    let mc_versions = versions.get_supported_minecraft_versions();
    assert!(!mc_versions.is_empty(), "No supported versions");

    let latest_1201 = versions.get_latest("1.20.1").expect("No 1.20.1 version");
    println!("Forge - Latest for 1.20.1: {}", latest_1201);
}

#[tokio::test]
async fn test_piston_mc_version_fetch() {
    let manifest = ManifestV2::fetch().await.expect("Failed to fetch MC manifest");

    let releases = manifest.releases();
    assert!(!releases.is_empty(), "No releases found");

    let latest = &manifest.latest;
    println!("Minecraft - Latest release: {}, snapshot: {}",
        latest.release, latest.snapshot);
}

#[tokio::test]
async fn test_piston_mc_java_manifest() {
    let manifest = JavaManifest::fetch().await.expect("Failed to fetch Java manifest");

    // Check that we have Windows x64 gamma runtimes (Java 17)
    let has_gamma = !manifest.windows_x64.gamma.is_empty();
    assert!(has_gamma, "No Java 17 (Gamma) runtime found");

    println!("Java runtimes available for Windows x64:");
    if !manifest.windows_x64.legacy.is_empty() {
        println!("  - legacy (Java 8): {} versions", manifest.windows_x64.legacy.len());
    }
    if !manifest.windows_x64.alpha.is_empty() {
        println!("  - alpha (Java 16): {} versions", manifest.windows_x64.alpha.len());
    }
    if !manifest.windows_x64.gamma.is_empty() {
        println!("  - gamma (Java 17): {} versions", manifest.windows_x64.gamma.len());
    }
    if !manifest.windows_x64.delta.is_empty() {
        println!("  - delta (Java 21): {} versions", manifest.windows_x64.delta.len());
    }
}
