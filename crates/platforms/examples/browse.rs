//! Searches and browses one platform — a live-network smoke test, run manually.
//!
//! Usage: `cargo run -p lodestone_platforms --example browse -- <modrinth|curseforge|atlauncher|ftb|technic> <query> <mod|modpack|shader|resourcepack|datapack|world> [api-key]`
//!
//! CurseForge requires an api-key; every other platform ignores it.

use lodestone_platforms::{AtLauncher, ContentType, CurseForge, Ftb, Modrinth, Platform, PlatformError, PlatformKind, SearchQuery, Technic};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(kind), Some(query), Some(content), credential) = (args.next(), args.next(), args.next(), args.next()) else {
        eprintln!("usage: browse <modrinth|curseforge|atlauncher|ftb|technic> <query> <mod|modpack|shader|resourcepack|datapack|world> [api-key]");
        std::process::exit(2);
    };

    let content = match content.as_str() {
        "mod" => ContentType::Mod,
        "modpack" => ContentType::Modpack,
        "shader" => ContentType::Shader,
        "resourcepack" => ContentType::ResourcePack,
        "datapack" => ContentType::DataPack,
        "world" => ContentType::World,
        other => {
            eprintln!("unknown content type {other:?}");
            std::process::exit(2);
        }
    };

    let result = match (kind.as_str(), credential.as_deref()) {
        ("modrinth", _) => browse(&Platform::<Modrinth>::new(), &query, content).await,
        ("curseforge", Some(key)) => browse(&Platform::<CurseForge>::with_api_key(key), &query, content).await,
        ("curseforge", None) => browse(&Platform::<CurseForge>::new(), &query, content).await,
        ("atlauncher", _) => browse(&Platform::<AtLauncher>::new(), &query, content).await,
        ("ftb", _) => browse(&Platform::<Ftb>::new(), &query, content).await,
        ("technic", _) => browse(&Platform::<Technic>::new(), &query, content).await,
        (other, _) => {
            eprintln!("unknown platform {other:?}");
            std::process::exit(2);
        }
    };

    match result {
        Ok(()) => {}
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}

async fn browse<K: PlatformKind>(platform: &Platform<K>, text: &str, content: ContentType) -> Result<(), PlatformError> {
    println!("searching {text:?} ({content:?})...");
    let page = platform
        .search(&SearchQuery {
            text: text.to_string(),
            ..SearchQuery::new(content)
        })
        .await?;
    let hits = page.hits;
    println!("{} of {} result(s)", hits.len(), page.total);
    for hit in hits.iter().take(5) {
        println!(
            "  [{}] {} by {} (downloads: {})",
            hit.id,
            hit.name,
            hit.author.as_deref().unwrap_or("?"),
            hit.downloads
        );
    }

    let Some(first) = hits.first() else { return Ok(()) };
    let details = platform.details(&first.id, content).await?;
    println!("\ndetails of {}:", details.name);
    println!("  slug: {}", details.slug.as_deref().unwrap_or("-"));
    println!("  summary: {}", details.summary);
    println!("  description: {} chars", details.description.as_deref().map(str::len).unwrap_or(0));

    let versions = platform.versions(&first.id, content).await?;
    println!("\n{} version(s)", versions.len());
    for version in versions.iter().take(5) {
        println!(
            "  [{}] {} (mc: {:?}, loaders: {:?}, files: {})",
            version.id,
            version.version_number,
            version.minecraft_versions.first().unwrap_or(&"-".to_string()),
            version.loaders,
            version.files.len()
        );
    }
    Ok(())
}
