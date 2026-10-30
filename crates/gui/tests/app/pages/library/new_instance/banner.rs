use std::collections::HashSet;

use super::*;

#[test]
fn every_scene_choice_maps_to_its_own_artwork_pair() {
    let pairs: HashSet<(Biome, u64)> = (0..SCENE_COUNT).map(scene).collect();
    assert_eq!(pairs.len(), SCENE_COUNT, "the 24 choices cover 8 biomes times 3 variants");
}

#[test]
fn random_scenes_stay_inside_the_picker() {
    for _ in 0..64 {
        assert!(random_scene() < SCENE_COUNT);
    }
}
