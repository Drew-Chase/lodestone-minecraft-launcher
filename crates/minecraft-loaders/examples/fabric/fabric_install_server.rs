use minecraft_modloaders::fabric::{FabricModLoader, FabricVersions};
use minecraft_modloaders::ModLoader;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Installing Fabric server...");

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

    // Setup paths
    let server_dir = PathBuf::from("./fabric-server");
    let java_path = PathBuf::from("java"); // Use system Java

    // You can also specify a custom Java path:
    // let java_path = PathBuf::from("C:/Program Files/Java/jdk-17/bin/java.exe");

    println!("\nServer Directory: {}", server_dir.display());
    println!("Java Path: {}", java_path.display());

    // Create server directory
    tokio::fs::create_dir_all(&server_dir).await?;

    println!("\nInstalling Fabric server...");
    println!("This may take a few minutes as it downloads Minecraft server JAR...");

    // Install the server using FabricModLoader
    let loader = FabricModLoader::new();
    let server_jar = loader
        .install_server(
            &game_version.version,
            &loader_version.version,
            &server_dir,
            &java_path,
        )
        .await?;

    println!("Successfully installed Fabric server!");
    println!("\nServer JAR location: {}", server_jar.display());
    println!("\nTo start the server, run:");
    println!("  java -jar {}", server_jar.display());

    Ok(())
}
