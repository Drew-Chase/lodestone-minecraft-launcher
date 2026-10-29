use super::*;
use crate::app::pages::library::instance::Biome;

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

#[tokio::test]
async fn schema_rolls_forward_and_round_trips() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    assert!(db.instances_empty().await.expect("empty check"), "a fresh db has no instances");

    let now = Local::now();
    let pack = Instance::seed_pack(now);
    let root = std::env::temp_dir().join(format!("lodestone-test-{}", std::process::id()));
    db.seed(&pack, &root).await.expect("seed succeeds");
    assert!(!db.instances_empty().await.expect("empty check"));

    let loaded = db.instances().await.expect("instances load");
    assert_eq!(loaded.len(), pack.len());
    for (original, restored) in pack.iter().zip(&loaded) {
        assert_eq!(original.id, restored.id);
        assert_eq!(original.name, restored.name);
        assert_eq!(original.mc_version, restored.mc_version);
        assert_eq!(original.loader, restored.loader);
        assert_eq!(original.loader_version, restored.loader_version);
        assert_eq!(original.playtime_seconds, restored.playtime_seconds);
        assert_eq!(original.mod_count, restored.mod_count);
        assert_eq!(original.pinned, restored.pinned);
        assert_eq!(original.state, restored.state);
        assert_eq!(original.biome, restored.biome);
        assert_eq!(original.art_seed, restored.art_seed);
        assert_eq!(original.directory_name, restored.directory_name);
        // Timestamps round-trip through unix seconds (sub-second precision dropped).
        assert_eq!(original.installed_at.timestamp(), restored.installed_at.timestamp());
        assert_eq!(
            original.last_played.map(|at| at.timestamp()),
            restored.last_played.map(|at| at.timestamp())
        );
    }
    assert!(loaded.iter().any(|instance| instance.biome == Biome::Nether));
    assert!(loaded.iter().any(|instance| instance.last_played.is_none()));

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn seeding_twice_fails_on_duplicate_ids() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let pack = Instance::seed_pack(Local::now());
    let root = std::env::temp_dir().join(format!("lodestone-test-{}-2", std::process::id()));
    db.seed(&pack, &root).await.expect("first seed succeeds");
    // The insert runs in a transaction — a duplicate id aborts the batch.
    assert!(db.seed(&pack, &root).await.is_err(), "re-seeding must not double-insert");
    std::fs::remove_dir_all(&root).ok();
}
