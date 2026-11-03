use super::*;

#[test]
fn a_project_page_names_its_slug() {
    let page = parse("https://www.curseforge.com/minecraft/modpacks/better-mc").expect("a modpack page");
    assert_eq!(
        page,
        ModpackPage {
            slug: "better-mc".to_string(),
            file_id: None,
        }
    );
    assert_eq!(page.address(), "https://www.curseforge.com/minecraft/modpacks/better-mc");
}

#[test]
fn a_file_page_names_its_file_too() {
    let page = parse("https://www.curseforge.com/minecraft/modpacks/better-mc/files/4123456").expect("a file page");
    assert_eq!(page.file_id.as_deref(), Some("4123456"));
    assert_eq!(page.address(), "https://www.curseforge.com/minecraft/modpacks/better-mc/files/4123456");
    assert_eq!(page.project_url(), "https://www.curseforge.com/minecraft/modpacks/better-mc");
}

#[test]
fn the_scheme_is_optional_and_a_trailing_slash_or_query_is_ignored() {
    assert_eq!(
        parse("curseforge.com/minecraft/modpacks/better-mc/").map(|page| page.slug),
        Some("better-mc".to_string())
    );
    assert_eq!(
        parse("  https://curseforge.com/minecraft/modpacks/better-mc?page=2  ").map(|page| page.slug),
        Some("better-mc".to_string())
    );
}

#[test]
fn other_hosts_games_pages_and_schemes_are_refused() {
    for text in [
        "https://www.example.com/minecraft/modpacks/better-mc",
        "https://curseforge.com.example.org/minecraft/modpacks/better-mc",
        "https://www.curseforge.com/minecraft/mc-mods/jei",
        "https://www.curseforge.com/minecraft/modpacks",
        "https://www.curseforge.com/minecraft/modpacks/better-mc/files/latest",
        "https://www.curseforge.com/minecraft/modpacks/better mc",
        "ftp://curseforge.com/minecraft/modpacks/better-mc",
        "not a url",
        "",
    ] {
        assert_eq!(parse(text), None, "{text} is not a modpack page");
    }
}
