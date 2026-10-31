use super::*;
use crate::app::pages::library::instance::{Biome, InstanceState, Loader, fixture};

impl Database {
    /// Opens an in-memory database with the migrations applied.
    async fn connect_in_memory() -> Result<Self, String> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .map_err(|error| error.to_string())?;
        sqlx::migrate!("./migrations").run(&pool).await.map_err(|error| error.to_string())?;
        Ok(Self { pool })
    }
}

/// A composed icon with the given catalog ids.
fn composed(background: &str, symbol: &str) -> InstanceIcon {
    InstanceIcon::Composed {
        background: background.to_string(),
        symbol: symbol.to_string(),
    }
}

#[tokio::test]
async fn a_fresh_database_lists_no_instances() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    assert!(db.instances().await.expect("instances load").is_empty());
}

#[tokio::test]
async fn instances_round_trip_through_the_table() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let mut played = fixture("0f3c-played", "Aether Legacy", Loader::Fabric);
    played.loader_version = Some("0.15.7".to_string());
    played.playtime_seconds = 24 * 3600 + 12 * 60;
    played.mod_count = 87;
    played.pinned = true;
    played.state = InstanceState::Playing;
    played.biome = Biome::Nether;
    played.art_seed = 42;
    played.last_played = Some(Local::now());
    let idle = fixture("9a1b-idle", "Vanilla Survival", Loader::Vanilla);
    for instance in [&played, &idle] {
        db.insert_instance(instance).await.expect("insert succeeds");
    }

    let loaded = db.instances().await.expect("instances load");
    assert_eq!(loaded.len(), 2);

    let restored = loaded.iter().find(|instance| instance.id == played.id).expect("played instance loads");
    assert_eq!(restored.name, played.name);
    assert_eq!(restored.mc_version, played.mc_version);
    assert_eq!(restored.loader, played.loader);
    assert_eq!(restored.loader_version, played.loader_version);
    assert_eq!(restored.playtime_seconds, played.playtime_seconds);
    assert_eq!(restored.mod_count, played.mod_count);
    assert_eq!(restored.pinned, played.pinned);
    assert_eq!(restored.state, played.state);
    assert_eq!(restored.biome, played.biome);
    assert_eq!(restored.art_seed, played.art_seed);
    assert_eq!(restored.directory_name, played.directory_name);
    // Timestamps round-trip through unix seconds (sub-second precision dropped).
    assert_eq!(restored.installed_at.timestamp(), played.installed_at.timestamp());
    assert_eq!(restored.last_played.map(|at| at.timestamp()), played.last_played.map(|at| at.timestamp()));

    let never = loaded.iter().find(|instance| instance.id == idle.id).expect("idle instance loads");
    assert!(never.last_played.is_none(), "an instance never played has no last-played time");
}

#[tokio::test]
async fn composed_icons_and_runtime_columns_round_trip() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let mut instance = fixture("0c1d-art", "Dragon Survival v2", Loader::NeoForge);
    instance.icon = composed("sunset", "fox");
    instance.art_path = Some(PathBuf::from(".lodestone/banner.png"));
    instance.java_component = Some("delta".to_string());
    instance.version_id = Some("neoforge-21.1.77".to_string());
    db.insert_instance(&instance).await.expect("insert succeeds");

    let restored = db.instances().await.expect("instances load").remove(0);
    assert_eq!(restored.icon, instance.icon);
    assert_eq!(restored.art_path, instance.art_path);
    assert_eq!(restored.java_component, instance.java_component);
    assert_eq!(restored.version_id, instance.version_id);
}

#[tokio::test]
async fn a_source_url_round_trips_and_is_empty_for_a_local_instance() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let mut modpack = fixture("0d2e-modpack", "Fabulously Optimized", Loader::Fabric);
    modpack.source_url = Some("https://modrinth.com/modpack/fabulously-optimized".to_string());
    db.insert_instance(&modpack).await.expect("insert succeeds");
    db.insert_instance(&fixture("0d2e-local", "Scratch", Loader::Vanilla))
        .await
        .expect("insert succeeds");

    let instances = db.instances().await.expect("instances load");
    let restored = instances
        .iter()
        .find(|instance| instance.id == "0d2e-modpack")
        .expect("the modpack loads");
    assert_eq!(restored.source_url, modpack.source_url);
    let local = instances
        .iter()
        .find(|instance| instance.id == "0d2e-local")
        .expect("the local instance loads");
    assert_eq!(local.source_url, None);
}

#[tokio::test]
async fn download_totals_count_today_and_the_days_of_the_week_only() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    db.add_download_bytes("2026-10-30", 100).await.expect("bytes add");
    db.add_download_bytes("2026-10-30", 50).await.expect("bytes add to the same day");
    db.add_download_bytes("2026-10-25", 1_000).await.expect("bytes add on an earlier day");
    db.add_download_bytes("2026-10-01", 5_000).await.expect("bytes add before the week");

    let totals = db.download_totals("2026-10-30", "2026-10-24").await.expect("totals read");
    assert_eq!(totals, DownloadTotals { today: 150, week: 1_150 });
}

#[tokio::test]
async fn uploaded_icons_round_trip_as_their_path() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let mut instance = fixture("7e2a-upload", "Uploaded", Loader::Quilt);
    instance.icon = InstanceIcon::Custom(PathBuf::from(".lodestone/icon.png"));
    db.insert_instance(&instance).await.expect("insert succeeds");

    let restored = db.instances().await.expect("instances load").remove(0);
    assert_eq!(restored.icon, InstanceIcon::Custom(PathBuf::from(".lodestone/icon.png")));
    assert!(restored.art_path.is_none(), "an instance without an upload has no banner path");
}

#[tokio::test]
async fn recent_icons_put_the_latest_first_without_repeats() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    db.record_recent_icon("rose", "grass").await.expect("first recent records");
    db.record_recent_icon("azure", "fox").await.expect("second recent records");
    db.record_recent_icon("rose", "grass").await.expect("a repeat records");

    let recents = db.recent_icons().await.expect("recents load");
    assert_eq!(recents, vec![composed("rose", "grass"), composed("azure", "fox")]);
}

#[tokio::test]
async fn recent_icons_are_capped() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    for index in 0..RECENT_ICON_LIMIT + 3 {
        db.record_recent_icon("rose", &format!("symbol-{index}")).await.expect("recent records");
    }

    let recents = db.recent_icons().await.expect("recents load");
    assert_eq!(recents.len(), RECENT_ICON_LIMIT as usize);
    assert_eq!(recents[0], composed("rose", &format!("symbol-{}", RECENT_ICON_LIMIT + 2)));
}

#[tokio::test]
async fn deleting_an_instance_removes_only_its_row() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let doomed = fixture("doomed", "Doomed", Loader::Vanilla);
    let kept = fixture("kept", "Kept", Loader::Fabric);
    db.insert_instance(&doomed).await.expect("insert succeeds");
    db.insert_instance(&kept).await.expect("insert succeeds");

    db.delete_instance(&doomed.id).await.expect("delete succeeds");

    let left = db.instances().await.expect("instances load");
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].id, kept.id);
}

#[tokio::test]
async fn deleting_an_unknown_instance_is_not_a_failure() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    db.delete_instance("nobody").await.expect("a row that is already gone is not an error");
}

#[tokio::test]
async fn migration_0002_removes_the_seeded_demo_rows_only() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let demo = fixture("seed-000", "Aether Legacy", Loader::Fabric);
    let real = fixture("3c9e4f0a-6b1d-4f7e-9c2a-8d5b1e7f0a42", "My Pack", Loader::Quilt);
    db.insert_instance(&demo).await.expect("demo row inserts");
    db.insert_instance(&real).await.expect("real row inserts");

    sqlx::query(include_str!("../../migrations/0002_remove_seed_instances.sql"))
        .execute(&db.pool)
        .await
        .expect("migration 0002 runs");

    let ids: Vec<String> = db
        .instances()
        .await
        .expect("instances load")
        .into_iter()
        .map(|instance| instance.id)
        .collect();
    assert_eq!(ids, std::slice::from_ref(&real.id));
}
