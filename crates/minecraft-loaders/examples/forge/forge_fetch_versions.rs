use minecraft_modloaders::forge::{ForgeEra, ForgeVersions};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Fetching Forge versions...\n");

    let versions = ForgeVersions::fetch().await?;

    // Get supported Minecraft versions
    let mc_versions = versions.get_supported_minecraft_versions();
    println!("=== Supported Minecraft Versions ({} total) ===", mc_versions.len());
    for (i, version) in mc_versions.iter().take(15).enumerate() {
        let era = ForgeEra::from_minecraft_version(version);
        let era_str = match era {
            ForgeEra::Legacy => "Legacy",
            ForgeEra::ArgsFile => "ArgsFile",
            ForgeEra::Modern => "Modern",
        };
        println!("  {}. {} ({})", i + 1, version, era_str);
    }
    if mc_versions.len() > 15 {
        println!("  ... and {} more", mc_versions.len() - 15);
    }

    // Show latest versions for some common Minecraft versions
    let sample_versions = ["1.20.1", "1.19.4", "1.18.2", "1.16.5", "1.12.2"];

    println!("\n=== Latest Forge Versions for Common MC Versions ===");
    for mc_version in sample_versions {
        if let Some(latest) = versions.get_latest(mc_version) {
            let era = ForgeEra::from_minecraft_version(mc_version);
            println!("  MC {} -> Forge {} ({:?})", mc_version, latest, era);
        } else {
            println!("  MC {} -> Not available", mc_version);
        }
    }

    // Show all Forge versions for 1.20.1
    println!("\n=== All Forge Versions for MC 1.20.1 ===");
    if let Some(forge_versions) = versions.get_versions("1.20.1") {
        for (i, version) in forge_versions.iter().take(10).enumerate() {
            println!("  {}. {}", i + 1, version);
        }
        if forge_versions.len() > 10 {
            println!("  ... and {} more", forge_versions.len() - 10);
        }
    }

    Ok(())
}
