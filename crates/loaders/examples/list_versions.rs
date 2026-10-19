//! Lists versions for one loader — a live-network smoke test, run manually.
//!
//! Usage: `cargo run -p lodestone_loaders --example list_versions -- <fabric|forge|neoforge|quilt|vanilla> [minecraft-version]`

use lodestone_loaders::{Fabric, Forge, Loader, LoaderError, LoaderKind, LoaderVersion, NeoForge, Quilt, Vanilla};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let Some(kind) = args.next() else {
        eprintln!("usage: list_versions <fabric|forge|neoforge|quilt|vanilla> [minecraft-version]");
        std::process::exit(2);
    };
    let minecraft = args.next();

    let result = match kind.as_str() {
        "fabric" => versions::<Fabric>(minecraft.as_deref()).await,
        "forge" => versions::<Forge>(minecraft.as_deref()).await,
        "neoforge" => versions::<NeoForge>(minecraft.as_deref()).await,
        "quilt" => versions::<Quilt>(minecraft.as_deref()).await,
        "vanilla" => versions::<Vanilla>(minecraft.as_deref()).await,
        other => {
            eprintln!("unknown loader {other:?}: use fabric, forge, neoforge, quilt, or vanilla");
            std::process::exit(2);
        }
    };

    match result {
        Ok(versions) => {
            println!("{} version(s):", versions.len());
            for version in versions {
                let minecraft = version.minecraft.as_deref().unwrap_or("-");
                let stable = if version.stable { "stable" } else { "unstable" };
                println!("  {} (minecraft: {minecraft}, {stable})", version.version);
            }
        }
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}

async fn versions<K: LoaderKind>(minecraft: Option<&str>) -> Result<Vec<LoaderVersion>, LoaderError> {
    let loader = Loader::<K>::new();
    match minecraft {
        Some(minecraft) => loader.versions_for_minecraft(minecraft).await,
        None => loader.list_versions().await,
    }
}
