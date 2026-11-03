use super::*;

/// A PNG of a 64×64 image filled with one colour.
fn skin_png(color: [u8; 4]) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(64, 64, image::Rgba(color));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).expect("encodes");
    bytes.into_inner()
}

/// A folder for one test. It is emptied first, so a run never sees what an earlier one left.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("lodestone-skins-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The names of the files in `dir`, in name order.
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

#[test]
fn the_archive_lives_beside_the_saved_skins() {
    assert_eq!(dir(Path::new("/root/skins")), PathBuf::from("/root/skins").join("archive"));
}

#[test]
fn a_replaced_skin_is_kept_once_in_each_model() {
    let dir = folder("kept");
    let png = skin_png([10, 200, 10, 255]);
    let digest = library::digest(&png);
    assert!(keep(&dir, &png, false, &[]).expect("writes"));
    assert!(
        !keep(&dir, &png, false, &[]).expect("checks"),
        "the same skin in the same model is kept once"
    );
    assert!(keep(&dir, &png, true, &[]).expect("writes"), "the model is part of the file's name");
    assert_eq!(names(&dir), vec![format!("{digest}-classic.png"), format!("{digest}-slim.png")]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_skin_the_saved_or_bundled_skins_have_is_not_archived() {
    let dir = folder("known");
    let png = skin_png([10, 200, 10, 255]);
    let known = vec![library::digest(&png)];
    assert!(!keep(&dir, &png, false, &known).expect("checks"));
    assert!(names(&dir).is_empty(), "nothing is written for a skin the player already has");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_partial_file_is_left_behind() {
    let dir = folder("partial");
    keep(&dir, &skin_png([1, 2, 3, 255]), false, &[]).expect("writes");
    assert!(names(&dir).iter().all(|name| name.ends_with(".png")), "{:?}", names(&dir));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_archive_reads_the_model_from_the_name() {
    let dir = folder("scan");
    keep(&dir, &skin_png([1, 2, 3, 255]), true, &[]).expect("writes");
    let skins = scan(&dir);
    assert_eq!(skins.len(), 1);
    assert_eq!(skins[0].slim, Some(true));
    assert!(!skins[0].name.ends_with(".png"), "a kept skin is named by its day, not its file");
    let _ = std::fs::remove_dir_all(&dir);
}
