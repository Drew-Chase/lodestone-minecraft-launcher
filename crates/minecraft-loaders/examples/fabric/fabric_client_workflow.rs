//! Full Fabric client workflow example.
//!
//! This example demonstrates the complete workflow for setting up and running
//! a Fabric Minecraft client:
//! 1. Fetch Fabric versions
//! 2. Download the correct Java version using piston-mc
//! 3. Download Minecraft assets using piston-mc
//! 4. Install the Fabric client
//! 5. Run the client for 30 seconds then stop it
//!
//! Note: This will launch the Minecraft client with full GUI.
//!
//! Run with: cargo run --example fabric_client_workflow

use crossterm::ExecutableCommand;
use crossterm::cursor::MoveUp;
use minecraft_modloaders::ModLoader;
use minecraft_modloaders::fabric::{FabricModLoader, FabricVersions};
use piston_mc::java::JavaManifest;
use piston_mc::manifest_v2::ManifestV2;
use piston_mc::version_manifest::LibraryItemDownloader;
use std::env::set_current_dir;
use std::io::{Write, stdout};
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    set_current_dir("target/").expect("Failed to set the target directory");
    println!("=== Fabric Client Full Workflow ===\n");

    // Setup paths
    let base_dir = PathBuf::from("./fabric-client-workflow");
    let java_dir = base_dir.join("java");
    let assets_dir = base_dir.join("assets");
    let instance_dir = base_dir.join("instance");
    // Libraries must be inside instance_dir for run_fabric_client to find them
    let library_dir = instance_dir.join("libraries");

    tokio::fs::create_dir_all(&java_dir).await?;
    tokio::fs::create_dir_all(&assets_dir).await?;
    tokio::fs::create_dir_all(&instance_dir).await?;
    tokio::fs::create_dir_all(&library_dir).await?;

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

    // Step 3: Download Minecraft assets
    println!("\nStep 3: Downloading Minecraft assets...");
    println!("  This may take a while...");
    let asset_index = download_assets(&mc_version, &assets_dir).await?;
    println!("  Assets downloaded to: {}", assets_dir.display());
    println!("  Asset index: {}", asset_index);

    // Step 4: Install Fabric client
    println!("\nStep 4: Installing Fabric client...");
    let loader = FabricModLoader::new();
    let client_path = instance_dir.join("client.jar");
    download_minecraft_client(&client_path, &library_dir, &mc_version).await?;

    loader
        .install_client(
            &mc_version,
            &loader_version,
            &instance_dir,
            &client_path,
            &java_path,
        )
        .await?;
    println!("  Client installed at: {}", client_path.display());

    // Step 5: Run the client
    println!("\nStep 5: Starting Fabric client (will run for 30 seconds)...");
    println!("  Note: A Minecraft window should appear.");
    println!("-------------------------------------------");

    let command = loader.run_fabric_client(
        &instance_dir,
        &client_path,
        &loader_version,
        &mc_version,
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
) -> anyhow::Result<Vec<PathBuf>> {
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
        let (sender, mut receiver) = tokio::sync::mpsc::channel(16);
        let library_result =
            version
                .libraries
                .download_with_client(&client, library_path, 150, Some(sender));
        let event_result = tokio::spawn(async move {
            let mut stdout = stdout();
            while let Some(message) = receiver.recv().await {
                let percentage: f32 = message.files_downloaded as f32 / message.files_total as f32;
                println!("\tLibrary Download progress: {:.2}%", percentage * 100.0);
                stdout.execute(MoveUp(1)).unwrap();
                if percentage >= 1.0 {
                    println!("\tLibrary Download complete!");
                    break;
                }
                stdout.flush().unwrap();
            }
        });
        let items = library_result.await?;
        event_result.await?;
        Ok(items)
    } else {
        Err(anyhow::Error::msg("No version found"))
    }
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
