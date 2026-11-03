use super::*;

#[test]
fn every_format_and_origin_is_stored_under_a_name_that_reads_back() {
    let formats = [
        ImportFormat::Mrpack,
        ImportFormat::CurseForge,
        ImportFormat::Lodestone,
        ImportFormat::PrismLauncher,
        ImportFormat::MultiMc,
        ImportFormat::AtLauncher,
        ImportFormat::MinecraftLauncher,
        ImportFormat::CurseForgeApp,
        ImportFormat::ModrinthApp,
    ];
    let names: std::collections::BTreeSet<&str> = formats.iter().map(|format| format.to_db()).collect();
    assert_eq!(names.len(), formats.len(), "each format has its own stored name");
    for format in formats {
        assert_eq!(ImportFormat::from_db(format.to_db()), Some(format));
    }
    for origin in [ImportOrigin::File, ImportOrigin::Url, ImportOrigin::Launcher] {
        assert_eq!(ImportOrigin::from_db(origin.to_db()), Some(origin));
    }
    assert_eq!(ImportFormat::from_db("unknown"), None);
    assert_eq!(ImportOrigin::from_db("unknown"), None);
}
