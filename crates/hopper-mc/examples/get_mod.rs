//! Fetch a single mod by id or slug from Modrinth and print its details.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example get_mod                 # defaults to "fabric-api"
//! cargo run --example get_mod -- sodium       # slug
//! cargo run --example get_mod -- AANobbMI     # project id
//! ```

use hopper_mc::{Platform, get_mod};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let slug = std::env::args().nth(1).unwrap_or_else(|| "fabric-api".into());

    match get_mod(&slug, Platform::Modrinth).await? {
        Some(m) => {
            println!("=== {} ({}) ===", m.base.title, m.base.slug);
            println!("Platform:     {:?}", m.base.platform);
            println!("Downloads:    {}", m.base.downloads);
            println!("Follows:      {}", m.base.follows);
            println!("Loaders:      {:?}", m.loaders);
            println!("MC versions:  {:?}", m.base.game_versions);
            println!("Categories:   {:?}", m.base.categories);
            println!("Client-side:  {:?}", m.client_side);
            println!("Server-side:  {:?}", m.server_side);
            if let Some(license) = &m.base.license {
                println!("License:      {}", license.id);
            }
            if let Some(src) = &m.base.links.source {
                println!("Source:       {src}");
            }
            if let Some(issues) = &m.base.links.issues {
                println!("Issues:       {issues}");
            }
            println!("\nSummary: {}", m.base.summary);
        }
        None => println!("Mod {slug:?} not found on Modrinth."),
    }

    Ok(())
}
