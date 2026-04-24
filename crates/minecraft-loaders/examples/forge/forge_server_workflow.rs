//! Full Forge server workflow example.
//!
//! This example demonstrates the complete workflow for setting up and running
//! a Forge Minecraft server:
//! 1. Fetch Forge versions
//! 2. Download the correct Java version using piston-mc
//! 3. Install the Forge server (downloads installer, runs it)
//! 4. Run the server for 30 seconds then stop it
//!
//! Run with: cargo run --example forge_server_workflow

use minecraft_modloaders::forge::{ForgeEra, ForgeModLoader, ForgeVersions};
use minecraft_modloaders::ModLoader;
use piston_mc::java::JavaManifest;
use piston_mc::manifest_v2::ManifestV2;
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Forge Server Full Workflow ===\n");

    // Setup paths
    let base_dir = PathBuf::from("./forge-server-workflow");
    let java_dir = base_dir.join("java");
    let server_dir = base_dir.join("server");

    tokio::fs::create_dir_all(&java_dir).await?;
    tokio::fs::create_dir_all(&server_dir).await?;

    // Use a well-tested Minecraft version
    let mc_version = "1.20.1";

    // Step 1: Fetch Forge versions
    println!("Step 1: Fetching Forge versions...");
    let forge_versions = ForgeVersions::fetch().await?;

    let forge_version = forge_versions
        .get_latest(mc_version)
        .ok_or_else(|| anyhow::anyhow!("No Forge version found for MC {}", mc_version))?
        .to_string();

    let era = ForgeEra::from_minecraft_version(mc_version);

    println!("  Minecraft: {}", mc_version);
    println!("  Forge: {}", forge_version);
    println!("  Era: {:?}", era);

    // Step 2: Download Java
    println!("\nStep 2: Downloading Java runtime...");
    let java_path = download_java(mc_version, &java_dir).await?;
    println!("  Java installed at: {}", java_path.display());

    // Step 3: Install Forge server
    println!("\nStep 3: Installing Forge server...");
    println!("  This may take several minutes as it downloads Minecraft and Forge libraries...");

    let loader = ForgeModLoader::new();
    let server_jar = loader
        .install_server(mc_version, &forge_version, &server_dir, &java_path)
        .await?;
    println!("  Server installed at: {}", server_jar.display());

    // Create eula.txt
    tokio::fs::write(server_dir.join("eula.txt"), "eula=true").await?;
    println!("  Created eula.txt");

    // Step 4: Run the server
    println!("\nStep 4: Starting Forge server (will run for 30 seconds)...");

    // Show era-specific run instructions
    match era {
        ForgeEra::Legacy => {
            println!("  Running in Legacy mode (direct JAR execution)");
        }
        ForgeEra::ArgsFile => {
            println!("  Running in ArgsFile mode (using @args.txt)");
        }
        ForgeEra::Modern => {
            println!("  Running in Modern mode (shim JAR)");
        }
    }

    println!("-------------------------------------------");

    let command = loader.run_server(
        &server_dir,
        &server_jar,
        &["-Xmx2G", "-Xms1G"],
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
    println!("\nTo manually start the server:");

    match era {
        ForgeEra::Legacy | ForgeEra::Modern => {
            println!("  java -jar {}", server_jar.display());
        }
        ForgeEra::ArgsFile => {
            println!(
                "  java @libraries/net/minecraftforge/forge/{}-{}/unix_args.txt nogui",
                mc_version, forge_version
            );
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
    runtime.install(install_dir, 4, None).await?;

    let java_path = if cfg!(windows) {
        install_dir.join("bin").join("java.exe")
    } else {
        install_dir.join("bin").join("java")
    };

    Ok(java_path)
}
