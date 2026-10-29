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
fn seed_pack_is_deterministic_and_varied() {
    let now = Local::now();
    let first = Instance::seed_pack(now);
    let second = Instance::seed_pack(now);
    assert_eq!(first.len(), 48);
    assert_eq!(format!("{first:?}"), format!("{second:?}"), "seed packs must be deterministic");

    let names: std::collections::HashSet<_> = first.iter().map(|instance| instance.name.clone()).collect();
    assert!(names.len() > 10, "seed names should vary, got {names:?}");
    let loaders: std::collections::HashSet<_> = first.iter().map(|instance| instance.loader).collect();
    assert_eq!(loaders.len(), 5, "every loader should appear");
    let biomes: std::collections::HashSet<_> = first.iter().map(|instance| instance.biome).collect();
    assert_eq!(biomes.len(), 8, "every biome should appear");
    assert!(
        first.iter().any(|instance| instance.state == InstanceState::Playing),
        "one instance is playing"
    );
    assert!(
        first.iter().any(|instance| instance.last_played.is_none()),
        "some instances were never played"
    );
    assert!(first.iter().all(|instance| !instance.directory_name.is_empty()));
}

#[test]
fn recency_prefers_last_played() {
    let now = Local::now();
    let mut pack = Instance::seed_pack(now);
    let mut instance = pack.remove(0);
    instance.last_played = None;
    assert_eq!(instance.recency(), instance.installed_at);
    instance.last_played = Some(now);
    assert_eq!(instance.recency(), now);
}
