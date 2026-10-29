use chrono::Local;

use super::*;

#[test]
fn loader_labels_and_db_round_trip() {
    for loader in [Loader::Vanilla, Loader::Fabric, Loader::Forge, Loader::Quilt, Loader::NeoForge] {
        assert_eq!(Loader::from_db(loader.to_db()), loader, "{loader:?} must survive the db encoding");
    }
    assert_eq!(Loader::Vanilla.label(), "VANILLA");
    assert_eq!(Loader::Fabric.label(), "FABRIC");
}

#[test]
fn biome_db_round_trip() {
    for biome in [
        Biome::End,
        Biome::Forest,
        Biome::Cherry,
        Biome::Ocean,
        Biome::Desert,
        Biome::Snow,
        Biome::Nether,
        Biome::Mushroom,
    ] {
        assert_eq!(Biome::from_db(biome.to_db()), biome, "{biome:?} must survive the db encoding");
    }
}

#[test]
fn recency_prefers_last_played() {
    let now = Local::now();
    let mut instance = fixture("aether", "Aether Legacy", Loader::Fabric);
    instance.last_played = None;
    assert_eq!(instance.recency(), instance.installed_at);
    instance.last_played = Some(now);
    assert_eq!(instance.recency(), now);
}
