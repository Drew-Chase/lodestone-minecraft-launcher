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

#[test]
fn icon_columns_round_trip_composed_and_custom() {
    let composed = InstanceIcon::Composed {
        background: "azure".to_string(),
        symbol: "fox".to_string(),
    };
    let (background, symbol, custom) = composed.to_db();
    assert_eq!(
        InstanceIcon::from_db(background.map(str::to_string), symbol.map(str::to_string), custom),
        composed
    );

    let custom = InstanceIcon::Custom(PathBuf::from(".lodestone/icon.png"));
    let (background, symbol, path) = custom.to_db();
    assert!(background.is_none() && symbol.is_none());
    assert_eq!(InstanceIcon::from_db(None, None, path), custom);
}

#[test]
fn resolve_files_makes_stored_paths_absolute() {
    let mut instance = fixture("0c1d", "Dragon Survival v2", Loader::Fabric);
    instance.directory_name = "Dragon Survival v2".to_string();
    instance.art_path = Some(PathBuf::from(".lodestone/banner.png"));
    instance.icon = InstanceIcon::Custom(PathBuf::from(".lodestone/icon.png"));

    instance.resolve_files(Path::new("/data/lodestone/instances"));

    let folder = Path::new("/data/lodestone/instances/Dragon Survival v2");
    assert_eq!(instance.art_path, Some(folder.join(".lodestone/banner.png")));
    assert_eq!(instance.icon, InstanceIcon::Custom(folder.join(".lodestone/icon.png")));
}
