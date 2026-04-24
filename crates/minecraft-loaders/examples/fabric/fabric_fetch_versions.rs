use minecraft_modloaders::fabric::FabricVersions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Fetching Fabric versions...");

    let versions = FabricVersions::fetch().await?;

    println!("\n=== Latest Stable Versions ===");

    if let Some(game) = versions.get_latest_game_version() {
        println!("Latest Game Version: {}", game.version);
    }

    if let Some(loader) = versions.get_latest_loader() {
        println!("Latest Loader Version: {} (build {})", loader.version, loader.build);
    }

    if let Some(installer) = versions.get_latest_installer() {
        println!("Latest Installer Version: {}", installer.version);
    }

    println!("\n=== Available Versions Count ===");
    println!("Game Versions: {}", versions.game.len());
    println!("Loader Versions: {}", versions.loader.len());
    println!("Installer Versions: {}", versions.installer.len());
    println!("Intermediary Versions: {}", versions.intermediary.len());

    println!("\n=== Recent Stable Game Versions ===");
    for game in versions.game.iter().filter(|g| g.stable).take(10) {
        println!("  - {}", game.version);
    }

    println!("\n=== Recent Stable Loader Versions ===");
    for loader in versions.loader.iter().filter(|l| l.stable).take(5) {
        println!("  - {} (build {})", loader.version, loader.build);
    }

    Ok(())
}
