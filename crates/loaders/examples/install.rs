//! Installs one loader version into a game folder, against the live APIs (run manually).
//!
//! ```text
//! cargo run -p lodestone_loaders --example install -- <fabric|quilt|forge|neoforge> <minecraft> <version> <game-dir> <java>
//! ```
//!
//! Forge and NeoForge run their installer with `<java>`, so it must be Java 17 or newer for
//! current versions. The installer's output is printed with a `|` prefix.

use std::error::Error;
use std::path::Path;

use lodestone_loaders::{Fabric, Forge, Loader, LoaderKind, NeoForge, Quilt};
use tokio::sync::mpsc::Sender;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [loader, minecraft, version, game_dir, java] = args.as_slice() else {
        return Err("usage: install <fabric|quilt|forge|neoforge> <minecraft> <version> <game-dir> <java>".into());
    };

    let (lines, mut printed) = tokio::sync::mpsc::channel::<String>(256);
    let printer = tokio::spawn(async move {
        while let Some(line) = printed.recv().await {
            println!("| {line}");
        }
    });
    let game_dir = Path::new(game_dir);
    let java = Path::new(java);
    match loader.as_str() {
        "fabric" => run::<Fabric>(minecraft, version, game_dir, java, &lines).await?,
        "quilt" => run::<Quilt>(minecraft, version, game_dir, java, &lines).await?,
        "forge" => run::<Forge>(minecraft, version, game_dir, java, &lines).await?,
        "neoforge" => run::<NeoForge>(minecraft, version, game_dir, java, &lines).await?,
        other => return Err(format!("unknown loader {other}").into()),
    }
    drop(lines);
    printer.await?;
    Ok(())
}

/// Finds `version` among the builds for `minecraft` and installs it into `game_dir`.
async fn run<K: LoaderKind>(minecraft: &str, version: &str, game_dir: &Path, java: &Path, lines: &Sender<String>) -> Result<(), Box<dyn Error>> {
    let loader = Loader::<K>::new();
    let found = loader
        .versions_for_minecraft(minecraft)
        .await?
        .into_iter()
        .find(|candidate| candidate.version == version)
        .ok_or_else(|| format!("{version} is not a {} build for Minecraft {minecraft}", K::NAME))?;
    let installed = loader.install(&found, game_dir, java, Some(lines), None).await?;
    println!("installed {} as {}", installed.id, installed.json.display());
    Ok(())
}
