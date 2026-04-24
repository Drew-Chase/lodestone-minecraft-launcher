use minecraft_modloaders::fabric::{FabricModLoader, FabricVersions};
use minecraft_modloaders::ModLoader;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Installing Fabric client...");

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
    let instance_dir = PathBuf::from("./fabric-instance");
    let client_path = instance_dir.join("client.jar");
    let java_path = PathBuf::from("java"); // Use system Java

    // You can also specify a custom Java path:
    // let java_path = PathBuf::from("C:/Program Files/Java/jdk-17/bin/java.exe");

    println!("\nInstance Directory: {}", instance_dir.display());
    println!("Java Path: {}", java_path.display());

    // Create instance directory
    tokio::fs::create_dir_all(&instance_dir).await?;

    println!("\nInstalling Fabric client...");
    println!("This may take a few minutes...");

    // Install the client using FabricModLoader
    let loader = FabricModLoader::new();
    let installed_path = loader
        .install_client(
            &game_version.version,
            &loader_version.version,
            &instance_dir,
            &client_path,
            &java_path,
        )
        .await?;

    println!("Successfully installed Fabric client!");
    println!("\nYou can now launch Minecraft with the Fabric profile from: {}", installed_path.display());

    Ok(())
}
