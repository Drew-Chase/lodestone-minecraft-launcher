//! Parses a modpack archive — works offline.
//!
//! Usage: `cargo run -p lodestone_platforms --example parse_modpack -- <modrinth|curseforge> <archive>`
//!
//! The platform marker only routes vanilla cases to their unsupported error;
//! the detected manifest format wins.

use lodestone_platforms::{CurseForge, Modrinth, Platform};

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(kind), Some(archive)) = (args.next(), args.next()) else {
        eprintln!("usage: parse_modpack <modrinth|curseforge> <archive>");
        std::process::exit(2);
    };

    let result = match kind.as_str() {
        "modrinth" => Platform::<Modrinth>::new().parse_modpack(&archive),
        "curseforge" => Platform::<CurseForge>::new().parse_modpack(&archive),
        other => {
            eprintln!("unknown platform {other:?}");
            std::process::exit(2);
        }
    };

    match result {
        Ok(modpack) => {
            println!("Name:           {}", modpack.name);
            println!("Version:        {}", modpack.version.as_deref().unwrap_or("-"));
            println!("Minecraft:      {}", modpack.minecraft);
            println!("Loader:         {}", modpack.loader.unwrap_or("-"));
            println!("Loader version: {}", modpack.loader_version.as_deref().unwrap_or("-"));
            println!("Overrides:      {}", modpack.overrides.as_deref().unwrap_or("-"));
            println!("Files:          {}", modpack.files.len());
            for file in modpack.files.iter().take(10) {
                if file.url.is_empty() {
                    println!(
                        "  ids {}:{} -> {}",
                        file.project_id.as_deref().unwrap_or("?"),
                        file.file_id.as_deref().unwrap_or("?"),
                        file.path
                    );
                } else {
                    println!("  {} ({} bytes)", file.path, file.size.unwrap_or(0));
                }
            }
        }
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
