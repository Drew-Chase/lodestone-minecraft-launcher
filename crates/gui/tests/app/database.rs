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

    /// Writes `instance` the way the New Instance flow will. Only the tests
    /// insert rows until that flow exists.
    async fn insert_for_test(&self, instance: &Instance) -> Result<(), String> {
        sqlx::query(
            "INSERT INTO instances \
             (id, name, mc_version, loader, loader_version, playtime_seconds, mod_count, \
              last_played_at, installed_at, pinned, state, biome, art_seed, directory_name) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&instance.id)
        .bind(&instance.name)
        .bind(&instance.mc_version)
        .bind(instance.loader.to_db())
        .bind(&instance.loader_version)
        .bind(instance.playtime_seconds as i64)
        .bind(instance.mod_count as i64)
        .bind(instance.last_played.map(|at| at.timestamp()))
        .bind(instance.installed_at.timestamp())
        .bind(instance.pinned as i64)
        .bind(instance.state.to_db())
        .bind(instance.biome.to_db())
        .bind(instance.art_seed as i64)
        .bind(&instance.directory_name)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
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
        db.insert_for_test(instance).await.expect("insert succeeds");
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
async fn migration_0002_removes_the_seeded_demo_rows_only() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let demo = fixture("seed-000", "Aether Legacy", Loader::Fabric);
    let real = fixture("3c9e4f0a-6b1d-4f7e-9c2a-8d5b1e7f0a42", "My Pack", Loader::Quilt);
    db.insert_for_test(&demo).await.expect("demo row inserts");
    db.insert_for_test(&real).await.expect("real row inserts");

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
