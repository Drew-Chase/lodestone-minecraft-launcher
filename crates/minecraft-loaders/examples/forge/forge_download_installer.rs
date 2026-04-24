use minecraft_modloaders::forge::{ForgeModLoader, ForgeVersions};
use minecraft_modloaders::ModLoader;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Downloading Forge installer...\n");

    // Fetch available versions
    let versions = ForgeVersions::fetch().await?;

    // Use 1.20.1 as an example
    let minecraft_version = "1.20.1";

    let forge_version = versions
        .get_latest(minecraft_version)
        .expect("No Forge version found for this Minecraft version");

    println!("Minecraft Version: {}", minecraft_version);
    println!("Forge Version: {}", forge_version);

    // Create output directory
    let output_dir = PathBuf::from("./forge-installer");
    tokio::fs::create_dir_all(&output_dir).await?;

    let output_path = output_dir.join(format!(
        "forge-{}-{}-installer.jar",
        minecraft_version, forge_version
    ));

    println!("\nDownloading to: {}", output_path.display());

    // Download the installer using ForgeModLoader
    // Note: Forge's download_server actually downloads the installer
    let loader = ForgeModLoader::new();
    let downloaded_path = loader
        .download_server(minecraft_version, forge_version, &output_path)
        .await?;

    println!("Successfully downloaded: {}", downloaded_path.display());

    // Get file size
    let metadata = tokio::fs::metadata(&downloaded_path).await?;
    let size_mb = metadata.len() as f64 / 1024.0 / 1024.0;
    println!("File size: {:.2} MB", size_mb);

    println!("\nTo install Forge server, run:");
    println!("  java -jar {} --installServer", downloaded_path.display());

    Ok(())
}
