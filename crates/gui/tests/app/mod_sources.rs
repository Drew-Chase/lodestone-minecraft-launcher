use super::*;

/// A modpack file at `path`, with the platform ids it names when given.
fn file(path: &str, project: Option<&str>, file: Option<&str>) -> ModpackFile {
    ModpackFile {
        path: path.to_string(),
        url: String::new(),
        sha1: Some("ab".repeat(20)),
        size: None,
        project_id: project.map(str::to_string),
        file_id: file.map(str::to_string),
    }
}

#[test]
fn a_mod_jar_is_a_file_directly_in_the_mods_folder() {
    assert_eq!(mod_jar_name("mods/sodium.jar"), Some("sodium.jar"));
    assert_eq!(mod_jar_name("mods/nested/sodium.jar"), None);
    assert_eq!(mod_jar_name("config/sodium.toml"), None);
    assert_eq!(mod_jar_name("mods/"), None);
}

#[test]
fn a_file_directly_in_a_folder_is_named_only_when_it_is_not_nested_deeper() {
    assert_eq!(direct_child("resourcepacks/Fresh.zip", "resourcepacks"), Some("Fresh.zip"));
    assert_eq!(direct_child("shaderpacks/Shade.zip", "shaderpacks"), Some("Shade.zip"));
    assert_eq!(direct_child("resourcepacks/nested/Fresh.zip", "resourcepacks"), None);
    assert_eq!(direct_child("shaderpacks/Shade.zip", "resourcepacks"), None);
    assert_eq!(direct_child("resourcepacks/", "resourcepacks"), None);
}

/// A placed file of a modpack, in `folder`, whose hash is `sha1`.
fn placed_file<'a>(folder: &'static str, file_name: &'a str, sha1: &str) -> Placed<'a> {
    Placed {
        folder,
        file_name,
        sha1: sha1.to_string(),
    }
}

/// The release a hash names on Modrinth, of `project_id`.
fn release(project_id: &str, version_id: &str) -> Release {
    Release {
        project_id: project_id.to_string(),
        version_id: version_id.to_string(),
    }
}

#[test]
fn a_mod_and_a_pack_are_recorded_under_their_own_folders() {
    let placed = [
        placed_file("mods", "sodium.jar", "aa"),
        placed_file("resourcepacks", "Fresh.zip", "bb"),
        placed_file("shaderpacks", "Shade.zip", "cc"),
    ];
    let releases = HashMap::from([
        ("aa".to_string(), release("p1", "v1")),
        ("bb".to_string(), release("p2", "v2")),
        ("cc".to_string(), release("p3", "v3")),
    ]);
    let names = HashMap::from([
        ("p1".to_string(), "Sodium".to_string()),
        ("p2".to_string(), "Fresh Animations".to_string()),
    ]);
    let records = records_of(&placed, &releases, &names);

    assert_eq!(records.mods.len(), 1);
    assert_eq!(records.mods[0].name, "Sodium");
    assert_eq!(records.mods[0].file_name, "sodium.jar");
    assert_eq!(records.mods[0].version_id, "v1");
    assert_eq!(records.packs.len(), 2);
    assert_eq!(records.packs[0].kind, "resourcepacks");
    assert_eq!(records.packs[0].platform, "modrinth");
    assert_eq!(records.packs[0].name, "Fresh Animations");
    assert_eq!(records.packs[1].kind, "shaderpacks");
    assert_eq!(records.packs[1].name, "Shade", "a pack whose project has no name keeps its file's name");
}

#[test]
fn a_file_no_release_names_is_recorded_as_nothing() {
    let placed = [placed_file("resourcepacks", "Own.zip", "dd")];
    let records = records_of(&placed, &HashMap::new(), &HashMap::new());
    assert!(records.mods.is_empty());
    assert!(records.packs.is_empty());
}

#[test]
fn a_stem_drops_only_the_last_extension() {
    assert_eq!(stem("sodium-0.5.8.jar"), "sodium-0.5.8");
    assert_eq!(stem("jar.disabled.jar"), "jar.disabled");
    assert_eq!(stem("plain"), "plain");
}

#[test]
fn curseforge_files_name_their_project_and_file_and_a_file_without_ids_is_skipped() {
    let files = [
        file("mods/jei.jar", Some("238222"), Some("4000")),
        file("mods/unnamed.jar", Some("1"), None),
        file("config/jei.toml", Some("238222"), Some("4000")),
    ];
    let records = curseforge_sources(&files);
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.platform, "curseforge");
    assert_eq!(record.project_id, "238222");
    assert_eq!(record.version_id, "4000");
    assert_eq!(record.name, "jei");
    assert_eq!(record.file_name, "jei.jar");
    assert_eq!(record.sha1, Some("ab".repeat(20)));
}

/// A scratch folder unique to this test run, holding the jars a test writes.
fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-mod-sources-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch folder created");
    dir
}

#[test]
fn a_jar_hashes_to_the_sha1_of_its_bytes() {
    let dir = scratch("digest");
    std::fs::write(dir.join("abc.jar"), b"abc").expect("jar written");
    assert_eq!(
        sha1_of_file(&dir.join("abc.jar")).as_deref(),
        Ok("a9993e364706816aba3e25717850c26c9cd0d89d")
    );
    std::fs::remove_dir_all(&dir).expect("scratch folder removed");
}

#[test]
fn a_cached_hash_stands_while_the_jar_keeps_its_size_and_time() {
    let dir = scratch("cache");
    std::fs::write(dir.join("a.jar"), b"abc").expect("jar written");
    let names = ["a.jar".to_string()];
    let (hashed, fresh) = hash_jars(&dir, &names, &HashMap::new());
    assert_eq!(
        hashed,
        vec![("a.jar".to_string(), "a9993e364706816aba3e25717850c26c9cd0d89d".to_string())]
    );
    assert_eq!(fresh.len(), 1, "a jar with no cached hash is hashed fresh");

    // The cache is trusted while the size and time match, so the bytes are not read again.
    let mut cached: HashMap<String, JarHash> = fresh.into_iter().map(|hash| (hash.file_name.clone(), hash)).collect();
    if let Some(hash) = cached.get_mut("a.jar") {
        hash.sha1 = "ff".repeat(20);
    }
    let (again, fresh_again) = hash_jars(&dir, &names, &cached);
    assert_eq!(again, vec![("a.jar".to_string(), "ff".repeat(20))]);
    assert!(fresh_again.is_empty());
    std::fs::remove_dir_all(&dir).expect("scratch folder removed");
}
