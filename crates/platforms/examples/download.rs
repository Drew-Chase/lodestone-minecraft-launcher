//! Downloads one version's files — a live-network smoke test, run manually.
//!
//! Usage: `cargo run -p lodestone_platforms --example download -- <modrinth|curseforge|atlauncher|ftb|technic> <project-id> <version-id> <content> <target-dir> [api-key]`

use lodestone_platforms::{AtLauncher, ContentType, CurseForge, Ftb, Modrinth, Platform, PlatformError, PlatformKind, Technic};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(kind), Some(project), Some(version_id), Some(content), Some(into)) = (args.next(), args.next(), args.next(), args.next(), args.next())
    else {
        eprintln!("usage: download <modrinth|curseforge|atlauncher|ftb|technic> <project-id> <version-id> <mod|modpack|...> <target-dir> [api-key]");
        std::process::exit(2);
    };
    let credential = args.next();
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
        ("modrinth", _) => run(&Platform::<Modrinth>::new(), &project, &version_id, content, &into).await,
        ("curseforge", Some(key)) => run(&Platform::<CurseForge>::with_api_key(key), &project, &version_id, content, &into).await,
        ("curseforge", None) => run(&Platform::<CurseForge>::new(), &project, &version_id, content, &into).await,
        ("atlauncher", _) => run(&Platform::<AtLauncher>::new(), &project, &version_id, content, &into).await,
        ("ftb", _) => run(&Platform::<Ftb>::new(), &project, &version_id, content, &into).await,
        ("technic", _) => run(&Platform::<Technic>::new(), &project, &version_id, content, &into).await,
        (other, _) => {
            eprintln!("unknown platform {other:?}");
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

/// Real flow: list the project's versions, pick the requested one, download
/// its files.
async fn run<K: PlatformKind>(
    platform: &Platform<K>,
    project: &str,
    version_id: &str,
    content: ContentType,
    into: &str,
) -> Result<Vec<std::path::PathBuf>, PlatformError> {
    let versions = platform.versions(project, content).await?;
    let version = versions.iter().find(|version| version.id == version_id);
    let Some(version) = version else {
        println!("version {version_id:?} not found among {} version(s)", versions.len());
        return Ok(Vec::new());
    };
    platform.download(version, into, None).await
}
