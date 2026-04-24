//! Search Modrinth for mods and print the first page of results.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example find_mods
//! cargo run --example find_mods -- "fabric api"
//! ```

use hopper_mc::{Platform, Sort, find_mods};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Optional CLI arg: a search query. If omitted, we browse by downloads.
    let query: Option<String> = std::env::args().nth(1);
    let query_ref = query.as_deref();

    let page = 0;
    let per_page = 10;

    let sort = if query_ref.is_some() {
        Sort::Relevance
    } else {
        Sort::Downloads
    };

    println!(
        "Searching Modrinth for mods (query = {:?}, sort = {:?})",
        query_ref, sort
    );

    let results = find_mods(query_ref, sort, Platform::Modrinth, page, per_page).await?;

    println!("Got {} result(s):\n", results.len());
    for (i, m) in results.iter().enumerate() {
        println!(
            "{:>2}. {}  [{}]",
            i + 1,
            m.base.title,
            m.base.slug
        );
        println!(
            "     downloads: {:>10}   loaders: {:?}",
            m.base.downloads, m.loaders
        );
        println!("     {}\n", m.base.summary);
    }

    Ok(())
}
