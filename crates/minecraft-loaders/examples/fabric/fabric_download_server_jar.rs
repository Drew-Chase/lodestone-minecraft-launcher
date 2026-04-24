use minecraft_modloaders::fabric::{FabricModLoader, FabricVersions};
use minecraft_modloaders::ModLoader;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Downloading Fabric server JAR...");

    // Fetch available versions
    let versions = FabricVersions::fetch().await?;

    // Get the latest stable versions
    let game_version = versions
        .get_latest_game_version()
        .expect("No stable game version found");
    let loader_version = versions
        .get_latest_loader()
        .expect("No stable loader version found");

    println!("Minecraft Version: {}", game_version.version);
    println!("Fabric Loader Version: {}", loader_version.version);

    // Create output directory
    let output_dir = PathBuf::from("./server");
    tokio::fs::create_dir_all(&output_dir).await?;

    let output_path = output_dir.join(format!(
        "fabric-server-{}-{}.jar",
        game_version.version, loader_version.version
    ));

    println!("\nDownloading to: {}", output_path.display());

    // Download the server JAR using FabricModLoader
    let loader = FabricModLoader::new();
    let downloaded_path = loader
        .download_server(&game_version.version, &loader_version.version, &output_path)
        .await?;

    println!("Successfully downloaded: {}", downloaded_path.display());

    // Get file size
    let metadata = tokio::fs::metadata(&downloaded_path).await?;
    let size_mb = metadata.len() as f64 / 1024.0 / 1024.0;
    println!("File size: {:.2} MB", size_mb);

    Ok(())
}
