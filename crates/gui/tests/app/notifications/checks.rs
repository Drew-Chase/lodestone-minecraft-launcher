use super::*;

/// A release called `version`, for the Minecraft versions `minecraft`.
fn release(version: &str, minecraft: &[&str]) -> Release {
    Release {
        version: version.to_owned(),
        minecraft: minecraft.iter().map(|minecraft| (*minecraft).to_owned()).collect(),
    }
}

#[test]
fn the_first_check_of_an_instance_records_its_newest_release_as_the_baseline() {
    let releases = [release("2.0", &["1.20.1"]), release("1.9", &["1.20.1"])];
    assert_eq!(pack_news(None, &releases, "1.20.1"), PackNews::Baseline("2.0".to_owned()));
}

#[test]
fn a_newer_release_for_the_instances_minecraft_version_is_an_update() {
    let releases = [release("2.0", &["1.20.1"]), release("1.9", &["1.20.1"])];
    assert_eq!(pack_news(Some("1.9"), &releases, "1.20.1"), PackNews::Update("2.0".to_owned()));
}

#[test]
fn a_newest_release_for_another_minecraft_version_is_not_an_update() {
    let releases = [release("2.0", &["1.21.1"]), release("1.9", &["1.20.1"])];
    assert_eq!(pack_news(Some("1.9"), &releases, "1.20.1"), PackNews::Nothing);
}

#[test]
fn the_installed_release_being_the_newest_is_nothing() {
    let releases = [release("2.0", &["1.20.1"])];
    assert_eq!(pack_news(Some("2.0"), &releases, "1.20.1"), PackNews::Nothing);
}

#[test]
fn a_pack_with_no_release_for_the_instance_is_nothing() {
    let releases = [release("2.0", &["1.21.1"])];
    assert_eq!(pack_news(Some("1.0"), &releases, "1.20.1"), PackNews::Nothing);
    assert_eq!(pack_news(None, &releases, "1.20.1"), PackNews::Nothing);
}

#[test]
fn a_mods_folder_lists_its_jar_files_by_name() {
    let folder = std::env::temp_dir().join(format!("lodestone-jars-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).expect("the folder is created");
    std::fs::write(folder.join("sodium.jar"), b"").expect("a jar is written");
    std::fs::write(folder.join("readme.txt"), b"").expect("a text file is written");
    let mut names = jar_names(&folder);
    names.sort();
    assert_eq!(names, ["sodium.jar"]);
    assert!(jar_names(&folder.join("missing")).is_empty());
    let _ = std::fs::remove_dir_all(&folder);
}
