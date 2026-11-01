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
