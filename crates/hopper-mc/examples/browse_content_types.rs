//! Browse every supported content type on Modrinth side-by-side and also
//! demonstrate what unimplemented / unsupported platforms return.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example browse_content_types
//! ```

use hopper_mc::{
    ContentError, Platform, SearchFilters, Sort, find_datapacks, find_mods, find_packs,
    find_resourcepacks, find_shaderpacks, find_worlds,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let page = 0;
    let per = 5;
    let sort = Sort::Downloads;
    let f = SearchFilters::default();

    // Modrinth supports everything except worlds.
    println!("--- Modrinth: top 5 of each kind ---\n");

    print_mods("Mods", find_mods(None, sort, &f, Platform::Modrinth, page, per).await?);
    print_packs("Modpacks", find_packs(None, sort, &f, Platform::Modrinth, page, per).await?);
    print_items(
        "Datapacks",
        find_datapacks(None, sort, &f, Platform::Modrinth, page, per)
            .await?
            .into_iter()
            .map(|d| (d.base.title, d.base.slug, d.base.downloads))
            .collect(),
    );
    print_items(
        "Resource packs",
        find_resourcepacks(None, sort, &f, Platform::Modrinth, page, per)
            .await?
            .into_iter()
            .map(|r| (r.base.title, r.base.slug, r.base.downloads))
            .collect(),
    );
    print_items(
        "Shader packs",
        find_shaderpacks(None, sort, &f, Platform::Modrinth, page, per)
            .await?
            .into_iter()
            .map(|s| (s.base.title, s.base.slug, s.base.downloads))
            .collect(),
    );

    // Worlds aren't served by Modrinth — the call returns UnsupportedContentType.
    match find_worlds(None, sort, &f, Platform::Modrinth, page, per).await {
        Ok(ws) => println!("Worlds (unexpected): {} results", ws.len()),
        Err(ContentError::UnsupportedContentType { platform, kind }) => {
            println!("As expected: {platform:?} does not serve {kind:?}");
        }
        Err(other) => eprintln!("Unexpected error on worlds: {other}"),
    }

    // CurseForge is a skeleton in this crate — calls return NotImplemented.
    match find_mods(None, sort, &f, Platform::CurseForge, 0, 1).await {
        Ok(_) => println!("CurseForge mods returned results (implementation is wired)"),
        Err(ContentError::NotImplemented(p)) => {
            println!("CurseForge mods: not yet implemented (platform = {p:?})");
        }
        Err(other) => eprintln!("Unexpected error from CurseForge: {other}"),
    }

    // AT Launcher / Technic / FTB only support modpacks; asking for mods
    // is an UnsupportedContentType at the dispatch layer (no network call).
    match find_mods(None, sort, &f, Platform::Ftb, 0, 1).await {
        Err(ContentError::UnsupportedContentType { platform, kind }) => {
            println!("FTB does not serve {kind:?} — platform {platform:?} is packs-only");
        }
        other => println!("FTB mods returned: {other:?}"),
    }

    Ok(())
}

fn print_mods(label: &str, items: Vec<hopper_mc::ModItem>) {
    println!("{label}:");
    for i in items {
        println!(
            "  {:<30} downloads={:>10} loaders={:?}",
            truncate(&i.base.title, 30),
            i.base.downloads,
            i.loaders
        );
    }
    println!();
}

fn print_packs(label: &str, items: Vec<hopper_mc::PackItem>) {
    println!("{label}:");
    for i in items {
        println!(
            "  {:<30} downloads={:>10} mc={:?}",
            truncate(&i.base.title, 30),
            i.base.downloads,
            i.mc_version
        );
    }
    println!();
}

fn print_items(label: &str, items: Vec<(String, String, u64)>) {
    println!("{label}:");
    for (title, slug, downloads) in items {
        println!(
            "  {:<30} [{}] downloads={}",
            truncate(&title, 30),
            slug,
            downloads
        );
    }
    println!();
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}
