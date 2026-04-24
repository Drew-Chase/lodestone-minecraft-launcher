//! Full Fabric server workflow example.
//!
//! This example demonstrates the complete workflow for setting up and running
//! a Fabric Minecraft server:
//! 1. Fetch Fabric versions
//! 2. Download the correct Java version using piston-mc
//! 3. Download the Fabric server JAR
//! 4. Run the server for 30 seconds then stop it
//!
//! Run with: cargo run --example fabric_server_workflow

use minecraft_modloaders::fabric::{FabricModLoader, FabricVersions};
use minecraft_modloaders::ModLoader;
use piston_mc::java::JavaManifest;
use piston_mc::manifest_v2::ManifestV2;
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Fabric Server Full Workflow ===\n");

    // Setup paths
    let base_dir = PathBuf::from("./fabric-server-workflow");
    let java_dir = base_dir.join("java");
    let server_dir = base_dir.join("server");

    tokio::fs::create_dir_all(&java_dir).await?;
    tokio::fs::create_dir_all(&server_dir).await?;

    // Step 1: Fetch Fabric versions
    println!("Step 1: Fetching Fabric versions...");
    let fabric_versions = FabricVersions::fetch().await?;

    let mc_version = fabric_versions
        .get_latest_game_version()
        .expect("No game version found")
        .version
        .clone();
    let loader_version = fabric_versions
        .get_latest_loader()
        .expect("No loader version found")
        .version
        .clone();

    println!("  Minecraft: {}", mc_version);
    println!("  Fabric Loader: {}", loader_version);

    // Step 2: Download Java
    println!("\nStep 2: Downloading Java runtime...");
    let java_path = download_java(&mc_version, &java_dir).await?;
    println!("  Java installed at: {}", java_path.display());

    // Step 3: Download Fabric server
    println!("\nStep 3: Downloading Fabric server...");
    let loader = FabricModLoader::new();
    let server_jar = server_dir.join(format!(
        "fabric-server-{}-{}.jar",
        mc_version, loader_version
    ));

    loader
        .download_server(&mc_version, &loader_version, &server_jar)
        .await?;
    println!("  Server JAR: {}", server_jar.display());

    // Create eula.txt
    tokio::fs::write(server_dir.join("eula.txt"), "eula=true").await?;
    println!("  Created eula.txt");

    // Step 4: Run the server
    println!("\nStep 4: Starting Fabric server (will run for 30 seconds)...");
    println!("-------------------------------------------");

    let command = loader.run_server(
        &server_dir,
        &server_jar,
        &["-Xmx1G", "-Xms512M"],
        &java_path,
    )?;

    let mut child = tokio::process::Command::from(command).spawn()?;

    // Let it run for 30 seconds
    tokio::time::sleep(Duration::from_secs(30)).await;

    println!("-------------------------------------------");
    println!("\nStopping server...");
    child.kill().await?;

    println!("\nWorkflow complete!");
    println!("Server directory: {}", server_dir.display());

    Ok(())
}

/// Download the correct Java version for a Minecraft version
async fn download_java(mc_version: &str, install_dir: &std::path::Path) -> anyhow::Result<PathBuf> {
    // Get the required Java version from Minecraft's version manifest
    let manifest = ManifestV2::fetch().await?;
    let version_manifest = manifest
        .version(mc_version)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Minecraft version {} not found", mc_version))?;

    let java_major = version_manifest
        .java_version
        .as_ref()
        .map(|jv| jv.major_version)
        .unwrap_or(17);

    println!("  Required Java version: {}", java_major);

    // Fetch Java manifest and select the appropriate runtime
    let java_manifest = JavaManifest::fetch().await?;

    let runtime = match java_major {
        8 => java_manifest.windows_x64.legacy.first(),
        16 => java_manifest.windows_x64.alpha.first(),
        17 => java_manifest.windows_x64.gamma.first(),
        21 => java_manifest.windows_x64.delta.first(),
        _ => java_manifest.windows_x64.gamma.first(),
    }
    .ok_or_else(|| anyhow::anyhow!("No Java runtime found for Java {}", java_major))?;

    println!("  Downloading Java runtime...");
    runtime.install(install_dir, 4, None).await?;

    let java_path = if cfg!(windows) {
        install_dir.join("bin").join("java.exe")
    } else {
        install_dir.join("bin").join("java")
    };

    Ok(java_path)
}
