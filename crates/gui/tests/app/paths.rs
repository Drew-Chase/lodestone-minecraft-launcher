use std::path::Path;

use super::*;

#[test]
fn every_folder_hangs_from_the_root() {
    let paths = LauncherPaths::new("/data/lodestone");
    assert_eq!(paths.instances(), Path::new("/data/lodestone/instances"));
    assert_eq!(
        paths.instance("Dragon Survival v2"),
        Path::new("/data/lodestone/instances/Dragon Survival v2")
    );
    assert_eq!(paths.java("delta"), Path::new("/data/lodestone/meta/java/delta"));
}

#[test]
fn shared_downloads_sit_under_meta_once() {
    let paths = LauncherPaths::new("/data/lodestone");
    assert_eq!(paths.libraries(), Path::new("/data/lodestone/meta/libraries"));
    assert_eq!(paths.assets(), Path::new("/data/lodestone/meta/assets"));
    assert_eq!(paths.asset_indexes(), Path::new("/data/lodestone/meta/assets/indexes"));
    assert_eq!(paths.asset_objects(), Path::new("/data/lodestone/meta/assets/objects"));
    assert_eq!(
        paths.version_json("1.21.4"),
        Path::new("/data/lodestone/meta/versions/1.21.4/1.21.4.json")
    );
    assert_eq!(paths.client_jar("1.21.4"), Path::new("/data/lodestone/meta/versions/1.21.4/1.21.4.jar"));
    assert_eq!(paths.natives("1.21.4"), Path::new("/data/lodestone/meta/natives/1.21.4"));
    assert_eq!(paths.log_configs(), Path::new("/data/lodestone/meta/log_configs"));
}
