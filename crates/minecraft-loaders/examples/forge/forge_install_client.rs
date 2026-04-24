use minecraft_modloaders::forge::{ForgeModLoader, ForgeVersions};
use minecraft_modloaders::ModLoader;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Installing Forge client...\n");

    // Fetch available versions
    let versions = ForgeVersions::fetch().await?;

    // Use 1.20.1 as an example
    let minecraft_version = "1.20.1";

    let forge_version = versions
        .get_latest(minecraft_version)
        .expect("No Forge version found for this Minecraft version");

    println!("Minecraft Version: {}", minecraft_version);
    println!("Forge Version: {}", forge_version);

    // Setup paths
    let instance_dir = PathBuf::from("./forge-instance");
    let client_path = instance_dir.join("client.jar");
    let java_path = PathBuf::from("java"); // Use system Java

    // You can also specify a custom Java path:
    // let java_path = PathBuf::from("C:/Program Files/Java/jdk-17/bin/java.exe");

    println!("\nInstance Directory: {}", instance_dir.display());
    println!("Java Path: {}", java_path.display());

    // Create instance directory
    tokio::fs::create_dir_all(&instance_dir).await?;

    println!("\nInstalling Forge client...");
    println!("This may take a few minutes...");
    println!("Note: This will open the Forge installer GUI.");

    // Install the client using ForgeModLoader
    let loader = ForgeModLoader::new();
    let installed_path = loader
        .install_client(
            minecraft_version,
            forge_version,
            &instance_dir,
            &client_path,
            &java_path,
        )
        .await?;

    println!("Successfully installed Forge client!");
    println!("\nInstallation path: {}", installed_path.display());
    println!("\nYou can now launch Minecraft with the Forge profile.");

    Ok(())
}
