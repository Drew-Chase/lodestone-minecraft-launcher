//! Write code that is generic over *which* platform serves mods.
//!
//! The top-level `find_mods(…, Platform, …)` dispatch is convenient, but
//! sometimes you want to hold a provider handle directly — for example when
//! your app has a user-selectable default platform, or when you want to
//! write a helper that works against any [`ModProvider`].
//!
//! Run with:
//!
//! ```sh
//! cargo run --example custom_provider_trait
//! ```

use hopper_mc::{
    ContentError, ModItem, ModProvider, ModrinthProvider, Sort,
};

/// Accept any provider that knows how to find mods. A simple generic
/// function that could just as easily live in library code.
async fn first_hit<P: ModProvider>(
    provider: &P,
    query: &str,
) -> Result<Option<ModItem>, ContentError> {
    let mut hits = provider.find_mods(Some(query), Sort::Relevance, 0, 1).await?;
    Ok(hits.pop())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Use `with_user_agent` to brand requests with your own app name —
    // Modrinth's ToS asks that clients identify themselves distinctly.
    let modrinth = ModrinthProvider::with_user_agent("my-launcher/1.0 (contact@example.com)");

    if let Some(m) = first_hit(&modrinth, "sodium").await? {
        println!("Top hit for \"sodium\":");
        println!("  title: {}", m.base.title);
        println!("  slug:  {}", m.base.slug);
        println!("  downloads: {}", m.base.downloads);
    } else {
        println!("No hit for \"sodium\".");
    }

    // Swap in a skeleton provider to show the NotImplemented path. This is
    // exactly what you'd do in production code once CurseForge support
    // is wired — callers don't change.
    use hopper_mc::CurseForgeProvider;
    let cf = CurseForgeProvider::new();
    match first_hit(&cf, "sodium").await {
        Ok(_) => println!("CurseForge returned a hit (implementation is now live)"),
        Err(ContentError::NotImplemented(p)) => {
            println!("CurseForge is a skeleton: NotImplemented({p:?})");
        }
        Err(e) => eprintln!("unexpected: {e}"),
    }

    Ok(())
}
