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

/// A mod recorded against `instance`, as the jar `file_name` of the project `project_id`.
fn source(instance: &str, project_id: &str, file_name: &str) -> ModSource {
    ModSource {
        instance: instance.to_string(),
        platform: "modrinth".to_string(),
        project_id: project_id.to_string(),
        version_id: format!("{project_id}-release"),
        name: format!("Project {project_id}"),
        file_name: file_name.to_string(),
        sha1: Some("ab".repeat(20)),
    }
}

/// A hash of the jar `file_name`, as the Mods tab caches it.
fn hash(file_name: &str) -> JarHash {
    JarHash {
        file_name: file_name.to_string(),
        size: 1_024,
        modified_at: 1_700_000_000,
        sha1: "cd".repeat(20),
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
        db.insert_instance(instance, &[]).await.expect("insert succeeds");
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
    db.insert_instance(&instance, &[]).await.expect("insert succeeds");

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
    db.insert_instance(&modpack, &[]).await.expect("insert succeeds");
    db.insert_instance(&fixture("0d2e-local", "Scratch", Loader::Vanilla), &[])
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
    db.insert_instance(&instance, &[]).await.expect("insert succeeds");

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
    db.insert_instance(&doomed, &[]).await.expect("insert succeeds");
    db.insert_instance(&kept, &[]).await.expect("insert succeeds");

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
async fn world_pins_round_trip_and_unpin() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    db.set_world_pin("aether", "Hub", true).await.expect("pin saves");
    db.set_world_pin("aether", "Hub", true).await.expect("a repeat pin is not an error");
    db.set_world_pin("aether", "Spawn", true).await.expect("pin saves");
    db.set_world_pin("aether", "Spawn", false).await.expect("unpin saves");

    let pins = db.world_pins().await.expect("pins load");
    assert_eq!(pins, [("aether".to_owned(), "Hub".to_owned())]);
}

#[tokio::test]
async fn deleting_an_instance_clears_its_world_pins_and_no_others() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let doomed = fixture("doomed", "Doomed", Loader::Vanilla);
    db.insert_instance(&doomed, &[]).await.expect("insert succeeds");
    db.set_world_pin("doomed", "Hub", true).await.expect("pin saves");
    db.set_world_pin("kept", "Hub", true).await.expect("pin saves");

    db.delete_instance("doomed").await.expect("delete succeeds");

    let pins = db.world_pins().await.expect("pins load");
    assert_eq!(pins, [("kept".to_owned(), "Hub".to_owned())]);
}

#[tokio::test]
async fn migration_0002_removes_the_seeded_demo_rows_only() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let demo = fixture("seed-000", "Aether Legacy", Loader::Fabric);
    let real = fixture("3c9e4f0a-6b1d-4f7e-9c2a-8d5b1e7f0a42", "My Pack", Loader::Quilt);
    db.insert_instance(&demo, &[]).await.expect("demo row inserts");
    db.insert_instance(&real, &[]).await.expect("real row inserts");

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

#[tokio::test]
async fn an_instance_inserted_with_its_mods_keeps_each_mod_against_it() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let installed = fixture("mods-a", "Modded", Loader::Fabric);
    let other = fixture("mods-b", "Other", Loader::Fabric);
    let sources = [source("mods-a", "sodium", "sodium.jar"), source("mods-a", "lithium", "lithium.jar")];
    db.insert_instance(&installed, &sources).await.expect("insert succeeds");
    db.insert_instance(&other, &[]).await.expect("insert succeeds");

    let mut recorded = db.mod_sources_of("mods-a").await.expect("mods load");
    recorded.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    assert_eq!(
        recorded,
        vec![source("mods-a", "lithium", "lithium.jar"), source("mods-a", "sodium", "sodium.jar")]
    );
    assert!(db.mod_sources_of("mods-b").await.expect("mods load").is_empty());
    assert_eq!(db.mod_sources().await.expect("every mod loads").len(), 2);
}

#[tokio::test]
async fn recording_a_jar_again_replaces_its_record() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    db.record_mod_sources(&[source("mods-a", "sodium", "sodium.jar")]).await.expect("records");
    let mut newer = source("mods-a", "sodium", "sodium.jar");
    newer.version_id = "sodium-newer".to_string();
    db.record_mod_sources(&[newer.clone()]).await.expect("records again");

    assert_eq!(db.mod_sources_of("mods-a").await.expect("mods load"), vec![newer]);
}

#[tokio::test]
async fn replacing_a_jar_moves_its_record_and_drops_its_cached_hash() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    db.record_mod_sources(&[source("mods-a", "sodium", "sodium-1.jar")])
        .await
        .expect("records");
    db.record_jar_hashes("mods-a", &[hash("sodium-1.jar")]).await.expect("hash caches");

    let replacement = source("mods-a", "sodium", "sodium-2.jar");
    db.replace_mod_source("sodium-1.jar", &replacement).await.expect("replaces");

    assert_eq!(db.mod_sources_of("mods-a").await.expect("mods load"), vec![replacement]);
    assert!(
        db.jar_hashes("mods-a").await.expect("hashes load").is_empty(),
        "the old jar's hash is gone"
    );
}

#[tokio::test]
async fn forgetting_a_jar_removes_its_record_and_its_hash() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    db.record_mod_sources(&[source("mods-a", "sodium", "sodium.jar"), source("mods-a", "lithium", "lithium.jar")])
        .await
        .expect("records");
    db.record_jar_hashes("mods-a", &[hash("sodium.jar"), hash("lithium.jar")])
        .await
        .expect("hashes cache");

    db.forget_mod_source("mods-a", "sodium.jar").await.expect("forgets");

    assert_eq!(
        db.mod_sources_of("mods-a").await.expect("mods load"),
        vec![source("mods-a", "lithium", "lithium.jar")]
    );
    assert_eq!(db.jar_hashes("mods-a").await.expect("hashes load"), vec![hash("lithium.jar")]);
}

#[tokio::test]
async fn forgetting_a_download_keeps_the_other_downloads_recorded() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let download = |project_id: &str| SharedRecord {
        kind: "resourcepacks".to_string(),
        platform: "modrinth".to_string(),
        project_id: project_id.to_string(),
        version_id: format!("{project_id}-release"),
        name: format!("Project {project_id}"),
        path: format!("modrinth-{project_id}-release"),
        size: Some(10),
    };
    db.record_shared_content(&download("chat")).await.expect("the download is recorded");
    db.record_shared_content(&download("fresh"))
        .await
        .expect("the other download is recorded");

    db.forget_shared_content("resourcepacks", "modrinth-chat-release")
        .await
        .expect("the download is forgotten");

    assert_eq!(db.shared_content().await.expect("records load"), vec![download("fresh")]);
}

#[tokio::test]
async fn cached_hashes_are_replaced_by_their_file_name() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    db.record_jar_hashes("mods-a", &[hash("a.jar")]).await.expect("caches");
    let mut changed = hash("a.jar");
    changed.size = 2_048;
    changed.sha1 = "ef".repeat(20);
    db.record_jar_hashes("mods-a", &[changed.clone()]).await.expect("caches again");

    assert_eq!(db.jar_hashes("mods-a").await.expect("hashes load"), vec![changed]);
    assert!(db.jar_hashes("mods-b").await.expect("hashes load").is_empty());
}

#[tokio::test]
async fn deleting_an_instance_clears_its_mod_records_and_hashes() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let doomed = fixture("mods-doomed", "Doomed", Loader::Fabric);
    db.insert_instance(&doomed, &[source("mods-doomed", "sodium", "sodium.jar")])
        .await
        .expect("insert succeeds");
    db.record_jar_hashes("mods-doomed", &[hash("sodium.jar")]).await.expect("caches");

    db.delete_instance("mods-doomed").await.expect("delete succeeds");

    assert!(db.mod_sources_of("mods-doomed").await.expect("mods load").is_empty());
    assert!(db.jar_hashes("mods-doomed").await.expect("hashes load").is_empty());
}

#[tokio::test]
async fn migration_0008_moves_each_discover_row_into_mod_sources() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("in-memory db opens");
    sqlx::query(include_str!("../../migrations/0007_instance_content.sql"))
        .execute(&pool)
        .await
        .expect("migration 0007 runs");
    sqlx::query("INSERT INTO instance_content VALUES ('mods-a', 'modrinth', 'sodium', 'sodium-release', 'Sodium', 'sodium.jar', 1700000000)")
        .execute(&pool)
        .await
        .expect("a discover row is written");

    sqlx::query(include_str!("../../migrations/0008_mod_sources.sql"))
        .execute(&pool)
        .await
        .expect("migration 0008 runs");

    let db = Database { pool };
    assert_eq!(
        db.mod_sources_of("mods-a").await.expect("mods load"),
        vec![ModSource {
            instance: "mods-a".to_string(),
            platform: "modrinth".to_string(),
            project_id: "sodium".to_string(),
            version_id: "sodium-release".to_string(),
            name: "Sodium".to_string(),
            file_name: "sodium.jar".to_string(),
            sha1: None,
        }]
    );
}

#[test]
fn an_unreadable_settings_value_means_every_setting_is_inherited() {
    assert_eq!(settings_of(None), InstanceSettings::default());
    assert_eq!(settings_of(Some("not json".to_string())), InstanceSettings::default());
}

#[tokio::test]
async fn an_edited_instance_is_saved_with_its_overrides_and_relative_pictures() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let mut instance = fixture("0f3c-tuned", "Aether Legacy", Loader::Fabric);
    instance.directory_name = "aether".to_string();
    db.insert_instance(&instance, &[]).await.expect("the row inserts");

    instance.name = "Tuned".to_string();
    instance.settings.override_with(crate::app::launch_settings::LaunchChange::Memory(8192));
    instance.art_path = Some(PathBuf::from("/instances/aether/.lodestone/banner-1.png"));
    instance.icon = InstanceIcon::Custom(PathBuf::from("/instances/aether/.lodestone/icon-1.png"));
    db.save_instance(&instance, Path::new("/instances/aether")).await.expect("the edit saves");

    let stored = db.instances().await.expect("instances load").into_iter().next().expect("one row");
    assert_eq!(stored.name, "Tuned");
    assert_eq!(stored.settings.memory_mb, Some(8192));
    assert_eq!(stored.art_path, Some(PathBuf::from(".lodestone/banner-1.png")));
    assert_eq!(stored.icon, InstanceIcon::Custom(PathBuf::from(".lodestone/icon-1.png")));
}

/// An import of `name` queued at `at`, as the Import dialog records it.
fn import(name: &str, at: DateTime<Local>) -> ImportRecord {
    ImportRecord {
        name: name.to_string(),
        format: ImportFormat::CurseForge,
        origin: ImportOrigin::Url,
        source: format!("https://www.curseforge.com/minecraft/modpacks/{name}"),
        bytes: 512,
        imported_at: at,
    }
}

#[tokio::test]
async fn recent_imports_are_the_newest_within_the_window() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let now = Local::now();
    for (name, days) in [("ten days", 10), ("six days", 6), ("two days", 2), ("one day", 1)] {
        db.record_import(&import(name, now - chrono::Duration::days(days)))
            .await
            .expect("the import records");
    }
    let recent = db.recent_imports(now - chrono::Duration::days(7), 3).await.expect("imports load");
    let names: Vec<&str> = recent.iter().map(|import| import.name.as_str()).collect();
    assert_eq!(names, ["one day", "two days", "six days"]);
    assert_eq!(recent[0].format, ImportFormat::CurseForge);
    assert_eq!(recent[0].origin, ImportOrigin::Url);
    assert_eq!(recent[0].bytes, 512);
}

#[tokio::test]
async fn only_the_newest_imports_are_kept() {
    let db = Database::connect_in_memory().await.expect("in-memory db opens");
    let now = Local::now();
    for index in 0..25 {
        let at = now - chrono::Duration::seconds(25 - index);
        db.record_import(&import(&format!("pack {index:02}"), at))
            .await
            .expect("the import records");
    }
    let kept = db.recent_imports(now - chrono::Duration::days(365), 100).await.expect("imports load");
    assert_eq!(kept.len(), 20);
    assert_eq!(kept[0].name, "pack 24");
    assert_eq!(kept[19].name, "pack 05");
}
