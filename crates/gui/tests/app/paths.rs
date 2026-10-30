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
    assert_eq!(paths.assets("1.21.4"), Path::new("/data/lodestone/meta/assets/1.21.4"));
}
