//! Prints the metadata of a mod jar — works offline.
//!
//! Usage: `cargo run -p lodestone_loaders --example mod_info -- <fabric|forge|neoforge|quilt> <mod.jar>`

use lodestone_loaders::{Fabric, Forge, Loader, NeoForge, Quilt};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(kind), Some(jar)) = (args.next(), args.next()) else {
        eprintln!("usage: mod_info <fabric|forge|neoforge|quilt> <mod.jar>");
        std::process::exit(2);
    };

    // The detected format wins regardless of the kind passed here; the marker
    // only routes vanilla to its unsupported error.
    let result = match kind.as_str() {
        "fabric" => Loader::<Fabric>::new().extract_mod_info(&jar),
        "forge" => Loader::<Forge>::new().extract_mod_info(&jar),
        "neoforge" => Loader::<NeoForge>::new().extract_mod_info(&jar),
        "quilt" => Loader::<Quilt>::new().extract_mod_info(&jar),
        other => {
            eprintln!("unknown loader {other:?}: use fabric, forge, neoforge, or quilt");
            std::process::exit(2);
        }
    };

    match result {
        Ok(info) => {
            println!("Loader:       {}", info.loader);
            println!("ID:           {}", info.id);
            println!("Version:      {}", info.version.as_deref().unwrap_or("-"));
            println!("Name:         {}", info.name.as_deref().unwrap_or("-"));
            println!("Description:  {}", info.description.as_deref().unwrap_or("-"));
            println!(
                "Authors:      {}",
                if info.authors.is_empty() {
                    "-".to_string()
                } else {
                    info.authors.join(", ")
                }
            );
            println!("License:      {}", info.license.as_deref().unwrap_or("-"));
            println!("Homepage:     {}", info.homepage.as_deref().unwrap_or("-"));
            if !info.dependencies.is_empty() {
                println!("Dependencies: {}", info.dependencies.len());
                for dependency in info.dependencies {
                    let range = dependency.version_range.as_deref().unwrap_or("*");
                    println!("  {:?} {} ({range})", dependency.kind, dependency.id);
                }
            }
        }
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
