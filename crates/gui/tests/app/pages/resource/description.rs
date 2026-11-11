use super::*;

#[test]
fn a_relative_link_out_unwraps_to_its_twice_encoded_target() {
    assert_eq!(
        resolve("/linkout?remoteUrl=https%253a%252f%252fdiscord.com%252finvite%252f72a7jmc5mb").as_deref(),
        Some("https://discord.com/invite/72a7jmc5mb")
    );
}

#[test]
fn a_link_out_encoded_once_or_on_another_curseforge_host_unwraps() {
    assert_eq!(
        resolve("https://www.curseforge.com/linkout?remoteUrl=https%3a%2f%2fneoforged.net%2f").as_deref(),
        Some("https://neoforged.net/")
    );
    assert_eq!(
        resolve("https://legacy.curseforge.com/linkout?remoteUrl=https%253a%252f%252fwww.patreon.com%252fx&foo=1").as_deref(),
        Some("https://www.patreon.com/x")
    );
}

#[test]
fn a_target_keeps_the_encoding_of_its_own_address() {
    assert_eq!(
        resolve("/linkout?remoteUrl=https%253a%252f%252fexample.com%252fa%252520b").as_deref(),
        Some("https://example.com/a%20b")
    );
}

#[test]
fn a_link_out_on_another_site_is_left_alone() {
    assert_eq!(
        resolve("https://evilcurseforge.com/linkout?remoteUrl=https%253a%252f%252fx.test").as_deref(),
        Some("https://evilcurseforge.com/linkout?remoteUrl=https%253a%252f%252fx.test")
    );
}

#[test]
fn a_link_out_to_something_that_is_not_a_web_address_is_rejected() {
    assert_eq!(resolve("/linkout?remoteUrl=javascript%253aalert(1)"), None);
}

#[test]
fn paths_and_scheme_relative_addresses_become_absolute() {
    assert_eq!(
        resolve("/minecraft/mc-mods/flywheel").as_deref(),
        Some("https://www.curseforge.com/minecraft/mc-mods/flywheel")
    );
    assert_eq!(resolve(" //i.imgur.com/a.png ").as_deref(), Some("https://i.imgur.com/a.png"));
    assert_eq!(
        resolve("HTTPS://Media.forgecdn.net/a.png").as_deref(),
        Some("HTTPS://Media.forgecdn.net/a.png")
    );
}

#[test]
fn addresses_that_are_not_web_addresses_are_rejected() {
    for href in [
        "javascript:alert(1)",
        "mailto:a@b.test",
        "#top",
        "data:image/png;base64,AA",
        "relative/page",
        "https://",
        "http:///x",
        "",
    ] {
        assert_eq!(resolve(href), None, "{href}");
    }
}
