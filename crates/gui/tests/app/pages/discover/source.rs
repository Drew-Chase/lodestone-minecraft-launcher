use super::*;
use std::collections::HashSet;

#[test]
fn every_platform_has_a_distinct_name_and_key_that_round_trip() {
    let names: HashSet<&str> = Source::ALL.iter().map(|source| source.name()).collect();
    let keys: HashSet<&str> = Source::ALL.iter().map(|source| source.key()).collect();
    assert_eq!(names.len(), 5);
    assert_eq!(keys.len(), 5);
    for source in Source::ALL {
        assert_eq!(Source::from_key(source.key()), Some(source));
    }
    assert_eq!(Source::from_key("all"), None, "there is no All platform");
}

#[test]
fn each_platform_hosts_the_content_its_capability_matrix_declares() {
    assert!(Source::Modrinth.supports(ContentType::Shader));
    assert!(Source::Modrinth.supports(ContentType::DataPack));
    assert!(!Source::Modrinth.supports(ContentType::World));
    assert!(Source::CurseForge.supports(ContentType::World));
    assert!(!Source::CurseForge.supports(ContentType::Shader));
    for pack_only in [Source::AtLauncher, Source::Technic, Source::Ftb] {
        assert!(pack_only.supports(ContentType::Modpack));
        assert!(!pack_only.supports(ContentType::ResourcePack));
    }
}

#[test]
fn the_keyless_platforms_are_searchable_in_every_build() {
    assert!(Source::Technic.enabled(), "Technic answers to its launcher name, with no key");
    assert!(Source::AtLauncher.enabled());
    assert!(Source::Modrinth.enabled());
    assert!(Source::Ftb.enabled());
}

#[test]
fn the_sorts_start_with_the_default_each_platform_can_express() {
    assert_eq!(Source::Modrinth.sorts()[0], SearchSort::Relevance);
    assert_eq!(Source::Ftb.sorts(), &[SearchSort::Popular]);
    assert!(Source::AtLauncher.sorts().contains(&SearchSort::Relevance));
}
