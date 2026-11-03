use super::*;

#[test]
fn every_bundled_skin_is_a_64_by_64_or_64_by_32_png() {
    for skin in BUNDLED {
        let image = image::load_from_memory(skin.png).unwrap_or_else(|error| panic!("{} is not a PNG: {error}", skin.name));
        assert_eq!(image.width(), 64, "{} is {}×{}", skin.name, image.width(), image.height());
        assert!(
            image.height() == 64 || image.height() == 32,
            "{} is {}×{}",
            skin.name,
            image.width(),
            image.height()
        );
    }
}

#[test]
fn the_defaults_are_the_eighteen_vanilla_skins_in_both_models() {
    let defaults: Vec<_> = BUNDLED.iter().filter(|skin| skin.pack == Pack::Defaults).collect();
    assert_eq!(defaults.len(), 18);
    assert_eq!(defaults.iter().filter(|skin| skin.slim).count(), 9);
}

#[test]
fn every_pack_but_the_defaults_has_a_title_and_the_defaults_do_not() {
    assert_eq!(Pack::Defaults.title(), None);
    for pack in Pack::ALL.into_iter().filter(|pack| *pack != Pack::Defaults) {
        assert!(pack.title().is_some_and(|title| !title.is_empty()), "{pack:?} needs a title");
    }
}

#[test]
fn every_pack_has_skins() {
    for pack in Pack::ALL {
        assert!(BUNDLED.iter().any(|skin| skin.pack == pack), "{pack:?} has no skins");
    }
}

#[test]
fn a_skin_is_named_once_per_pack_and_model() {
    let mut seen = std::collections::HashSet::new();
    for skin in BUNDLED {
        assert!(seen.insert((skin.pack, skin.name, skin.slim)), "{} appears twice", skin.name);
    }
}

#[test]
fn the_bundle_holds_the_forty_eight_skins_mojang_serves() {
    // The 41 pack skins that Mojang's texture server does not serve, and whose sources are unconfirmed, are left out.
    assert_eq!(BUNDLED.len(), 48);
}
