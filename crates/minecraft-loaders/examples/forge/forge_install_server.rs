use minecraft_modloaders::forge::{ForgeEra, ForgeModLoader, ForgeVersions};
use minecraft_modloaders::ModLoader;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Installing Forge server...\n");

    // Fetch available versions
    let versions = ForgeVersions::fetch().await?;

    // Use 1.20.1 as an example
    let minecraft_version = "1.20.1";

    let forge_version = versions
        .get_latest(minecraft_version)
        .expect("No Forge version found for this Minecraft version");

    let era = ForgeEra::from_minecraft_version(minecraft_version);

    println!("Minecraft Version: {}", minecraft_version);
    println!("Forge Version: {}", forge_version);
    println!("Forge Era: {:?}", era);

    // Setup paths
    let server_dir = PathBuf::from("./forge-server");
    let java_path = PathBuf::from("java"); // Use system Java

    // You can also specify a custom Java path:
    // let java_path = PathBuf::from("C:/Program Files/Java/jdk-17/bin/java.exe");

    println!("\nServer Directory: {}", server_dir.display());
    println!("Java Path: {}", java_path.display());

    // Create server directory
    tokio::fs::create_dir_all(&server_dir).await?;

    println!("\nInstalling Forge server...");
    println!("This may take a few minutes as it downloads the Minecraft server JAR...");

    // Install the server using ForgeModLoader
    let loader = ForgeModLoader::new();
    let server_jar = loader
        .install_server(
            minecraft_version,
            forge_version,
            &server_dir,
            &java_path,
        )
        .await?;

    println!("Successfully installed Forge server!");
    println!("\nServer JAR location: {}", server_jar.display());

    // Show how to run based on era
    match era {
        ForgeEra::Legacy => {
            println!("\nTo start the server (Legacy mode), run:");
            println!("  java -jar {}", server_jar.display());
        }
        ForgeEra::ArgsFile => {
            println!("\nTo start the server (ArgsFile mode), run:");
            println!("  java @libraries/net/minecraftforge/forge/{}-{}/unix_args.txt nogui",
                minecraft_version, forge_version);
            println!("\nOr on Windows:");
            println!("  java @libraries/net/minecraftforge/forge/{}-{}/win_args.txt nogui",
                minecraft_version, forge_version);
        }
        ForgeEra::Modern => {
            println!("\nTo start the server (Modern mode), run:");
            println!("  java -jar {}", server_jar.display());
        }
    }

    Ok(())
}
