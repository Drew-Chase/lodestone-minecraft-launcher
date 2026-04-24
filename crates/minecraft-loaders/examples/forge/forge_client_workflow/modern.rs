//! Full Forge client workflow example for Modern Forge (MC ≥1.20.3).
//!
//! This example demonstrates the complete workflow for setting up and running
//! a Modern Forge Minecraft client (1.21.1):
//! 1. Fetch Forge versions
//! 2. Download the correct Java version using piston-mc
//! 3. Download Minecraft assets using piston-mc
//! 4. Install the Forge client (headless - no GUI)
//! 5. Run the client for 30 seconds then stop it
//!
//! **KNOWN LIMITATION**: Modern Forge (≥1.20.3) uses a complex module-path
//! based launch configuration and BootstrapLauncher that requires additional setup.
//! The client installation works, but launching fails with class loading errors.
//! Legacy Forge (≤1.16.5) works correctly.
//!
//! Run with: cargo run --example forge_client_workflow_modern

use std::env::set_current_dir;
use std::io::{stdout, Write};
use minecraft_modloaders::forge::{ForgeModLoader, ForgeVersions};
use minecraft_modloaders::ModLoader;
use piston_mc::java::JavaManifest;
use piston_mc::manifest_v2::ManifestV2;
use std::path::PathBuf;
use std::time::Duration;
use crossterm::cursor::MoveUp;
use crossterm::ExecutableCommand;
use piston_mc::version_manifest::LibraryItemDownloader;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    set_current_dir("target/").expect("Failed to set the target directory");
    println!("=== Forge Client Full Workflow ===\n");

    // Setup paths
    let base_dir = PathBuf::from("./forge-client-workflow");
    let java_dir = base_dir.join("java");
    let assets_dir = base_dir.join("assets");
    let instance_dir = base_dir.join("instance");
    // Libraries must be inside instance_dir for run_forge_client to find them
    let library_dir = instance_dir.join("libraries");

    tokio::fs::create_dir_all(&java_dir).await?;
    tokio::fs::create_dir_all(&assets_dir).await?;
    tokio::fs::create_dir_all(&instance_dir).await?;
    tokio::fs::create_dir_all(&library_dir).await?;

    // Use a well-tested Minecraft version
    let mc_version = "1.21.1";

    // Step 1: Fetch Forge versions
    println!("Step 1: Fetching Forge versions...");
    let forge_versions = ForgeVersions::fetch().await?;

    let forge_version = forge_versions
        .get_latest(mc_version)
        .ok_or_else(|| anyhow::anyhow!("No Forge version found for MC {}", mc_version))?
        .to_string();

    println!("  Minecraft: {}", mc_version);
    println!("  Forge: {}", forge_version);

    // Step 2: Download Java
    println!("\nStep 2: Downloading Java runtime...");
    let java_path = download_java(mc_version, &java_dir).await?;
    println!("  Java installed at: {}", java_path.display());

    // Step 3: Download Minecraft assets
    println!("\nStep 3: Downloading Minecraft assets...");
    println!("  This may take a while...");
    let asset_index = download_assets(mc_version, &assets_dir).await?;
    println!("  Assets downloaded to: {}", assets_dir.display());
    println!("  Asset index: {}", asset_index);

    // Step 4: Install Forge client
    println!("\nStep 4: Installing Forge client...");

    let loader = ForgeModLoader::new();
    let client_path = instance_dir.join("client.jar");
    download_minecraft_client(&client_path, &library_dir, &mc_version.to_string()).await?;

    let _installed_path = loader
        .install_client(
            mc_version,
            &forge_version,
            &library_dir,
            &client_path,
            &java_path,
        )
        .await?;
    println!("  Client installed at: {}", client_path.display());

    // Step 5: Run the client
    println!("\nStep 5: Starting Forge client (will run for 30 seconds)...");
    println!("  Note: A Minecraft window should appear.");
    println!("-------------------------------------------");

    let command = loader.run_forge_client(
        &instance_dir,
        &client_path,
        mc_version,
        &forge_version,
        &["-Xmx2G", "-Xms512M"],
        &java_path,
        &assets_dir,
        &asset_index,
        &instance_dir, // game_dir - use instance_dir for saves, configs, etc.
    )?;

    let mut child = tokio::process::Command::from(command).spawn()?;

    // Let it run for 30 seconds
    tokio::time::sleep(Duration::from_secs(30)).await;

    println!("-------------------------------------------");
    println!("\nStopping client...");
    child.kill().await?;

    println!("\nWorkflow complete!");
    println!("Instance directory: {}", instance_dir.display());

    Ok(())
}

async fn download_minecraft_client(
    client_path: &PathBuf,
    library_path: &PathBuf,
    mc_version: &String,
) -> anyhow::Result<()> {
    let manifest = ManifestV2::fetch().await?;
    let version = manifest.version(&mc_version).await?;
    let client: reqwest::Client = reqwest::Client::new();
    if let Some(version) = version {
        {
            let client_url = version.downloads.client.url;

            let (sender, mut receiver) = tokio::sync::mpsc::channel(16);
            let download_result = simple_download_utility::download_file_with_client(
                &client,
                client_url,
                client_path,
                Some(sender),
            );
            let event_result = tokio::spawn(async move {
                let mut stdout = stdout();
                while let Some(message) = receiver.recv().await {
                    let percentage: f32 =
                        message.bytes_downloaded as f32 / message.bytes_to_download as f32;
                    println!("\tClient Download progress: {:.2}%", percentage * 100.0);
                    stdout.execute(MoveUp(1)).unwrap();
                    if percentage >= 1.0 {
                        println!("\tClient Download complete!");
                        break;
                    }
                    stdout.flush().unwrap();
                }
            });
            download_result.await?;
            event_result.await?;
        }
        {
            let (sender, mut receiver) = tokio::sync::mpsc::channel(16);
            let library_result =
                version
                    .libraries
                    .download_with_client(&client, library_path, 150, Some(sender));
            let event_result = tokio::spawn(async move {
                let mut stdout = stdout();
                while let Some(message) = receiver.recv().await {
                    let percentage: f32 =
                        message.files_downloaded as f32 / message.files_total as f32;
                    println!("\tLibrary Download progress: {:.2}%", percentage * 100.0);
                    stdout.execute(MoveUp(1)).unwrap();
                    if percentage >= 1.0 {
                        println!("\tLibrary Download complete!");
                        break;
                    }
                    stdout.flush().unwrap();
                }
            });
            library_result.await?;
            event_result.await?;
        }
    }
    Ok(())
}

/// Download the correct Java version for a Minecraft version
async fn download_java(mc_version: &str, install_dir: &std::path::Path) -> anyhow::Result<PathBuf> {
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
    let (sender, mut receiver) = tokio::sync::mpsc::channel(16);
    let install_result = runtime.install(install_dir, 20, Some(sender));
    let event_result = tokio::spawn(async move {
        let mut stdout = stdout();
        while let Some(message) = receiver.recv().await {
            let percentage: f32 = message.files_downloaded as f32 / message.files_total as f32;
            println!("\tJava Download progress: {:.2}%", percentage * 100.0);
            stdout.execute(MoveUp(1)).unwrap();
            if percentage >= 1.0 {
                println!("\tJava Download complete!");
                break;
            }
            stdout.flush().unwrap();
        }
    });

    install_result.await?;
    event_result.await?;

    let java_path = if cfg!(windows) {
        install_dir.join("bin").join("java.exe")
    } else {
        install_dir.join("bin").join("java")
    };

    Ok(java_path)
}

/// Download Minecraft assets for a specific version
/// Returns the asset index name (needed for launching the game)
async fn download_assets(mc_version: &str, assets_dir: &std::path::Path) -> anyhow::Result<String> {
    let manifest = ManifestV2::fetch().await?;
    let version_manifest = manifest
        .version(mc_version)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Minecraft version {} not found", mc_version))?;

    // Get the asset index name before consuming the assets
    let asset_index = version_manifest.assets.clone();

    let mut assets = version_manifest.assets().await?;
    let (sender, mut receiver) = tokio::sync::mpsc::channel(16);
    let asset_result = assets.download(assets_dir, 100, Some(sender));
    let event_result = tokio::spawn(async move {
        let mut stdout = stdout();
        while let Some(message) = receiver.recv().await {
            let percentage: f32 = message.files_downloaded as f32 / message.files_total as f32;
            println!("\tAsset Download progress: {:.2}%", percentage * 100.0);
            stdout.execute(MoveUp(1)).unwrap();
            if percentage >= 1.0 {
                println!("\tAsset Download complete!");
                break;
            }
            stdout.flush().unwrap();
        }
    });
    asset_result.await?;
    event_result.await?;

    Ok(asset_index)
}
