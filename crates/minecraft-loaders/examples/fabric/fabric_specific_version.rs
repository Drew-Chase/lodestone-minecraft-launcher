use minecraft_modloaders::fabric::{FabricModLoader, FabricVersions};
use minecraft_modloaders::ModLoader;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Working with specific Fabric versions...");

    // Fetch available versions
    let versions = FabricVersions::fetch().await?;

    // Specify the versions you want
    let minecraft_version = "1.20.1";
    let loader_version_string = "0.15.0";

    println!("\n=== Finding Specific Versions ===");
    println!("Looking for Minecraft: {}", minecraft_version);
    println!("Looking for Loader: {}", loader_version_string);

    // Find specific versions
    let game_version = versions
        .find_game_version(minecraft_version)
        .unwrap_or_else(|| panic!("Minecraft version {} not found", minecraft_version));

    let loader_version = versions
        .find_loader(loader_version_string)
        .or_else(|| {
            println!("Exact loader version not found, using latest stable...");
            versions.get_latest_loader()
        })
        .expect("No loader version found");

    println!("\n=== Found Versions ===");
    println!("Minecraft: {} (stable: {})", game_version.version, game_version.stable);
    println!("Loader: {} (stable: {})", loader_version.version, loader_version.stable);

    // Download server JAR for specific version
    let output_dir = PathBuf::from("./specific-server");
    tokio::fs::create_dir_all(&output_dir).await?;

    let output_path = output_dir.join(format!(
        "fabric-server-{}-{}.jar",
        game_version.version, loader_version.version
    ));

    println!("\n=== Downloading Server JAR ===");
    println!("Output: {}", output_path.display());

    // Download using FabricModLoader
    let loader = FabricModLoader::new();
    let downloaded_path = loader
        .download_server(&game_version.version, &loader_version.version, &output_path)
        .await?;

    println!("Successfully downloaded: {}", downloaded_path.display());

    // List all available versions for reference
    println!("\n=== Available Loader Versions (first 10) ===");
    for (i, loader) in versions.loader.iter().take(10).enumerate() {
        println!("  {}. {} (build: {}, stable: {})",
            i + 1, loader.version, loader.build, loader.stable);
    }

    Ok(())
}
