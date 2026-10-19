//! Downloads one loader version's client libraries — a live-network smoke test,
//! run manually.
//!
//! Usage: `cargo run -p lodestone_loaders --example download_client -- <fabric|forge|neoforge|quilt|vanilla> <version> <minecraft-version> <target-dir>`

use lodestone_loaders::{Fabric, Forge, Loader, LoaderVersion, NeoForge, Quilt, Vanilla};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(kind), Some(version), Some(minecraft), Some(into)) = (args.next(), args.next(), args.next(), args.next()) else {
        eprintln!("usage: download_client <fabric|forge|neoforge|quilt|vanilla> <version> <minecraft-version> <target-dir>");
        std::process::exit(2);
    };

    // Real callers get versions from versions_for_minecraft; the example binds
    // the version to the Minecraft version given on the command line.
    let version = LoaderVersion {
        version,
        minecraft: Some(minecraft),
        stable: true,
        changelog_url: None,
    };

    let result = match kind.as_str() {
        "fabric" => Loader::<Fabric>::new().download_client(&version, &into, None).await,
        "forge" => Loader::<Forge>::new().download_client(&version, &into, None).await,
        "neoforge" => Loader::<NeoForge>::new().download_client(&version, &into, None).await,
        "quilt" => Loader::<Quilt>::new().download_client(&version, &into, None).await,
        "vanilla" => Loader::<Vanilla>::new().download_client(&version, &into, None).await,
        other => {
            eprintln!("unknown loader {other:?}: use fabric, forge, neoforge, quilt, or vanilla");
            std::process::exit(2);
        }
    };

    match result {
        Ok(files) => {
            println!("downloaded {} file(s):", files.len());
            for file in files {
                println!("  {}", file.display());
            }
        }
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
