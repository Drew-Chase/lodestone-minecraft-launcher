//! Seeded catalogues for the instance tabs — the single data seam.
//!
//! Every list is deterministic for a given instance. Aether Legacy carries the
//! rows the reference images show; other instances get generated rows sized
//! from their own data. Replacing a function with a folder scan later leaves
//! the tab views untouched.

use chrono::{DateTime, Duration, Local, TimeZone};

use crate::app::pages::library::instance::{Biome, Instance, Loader};

use super::format::format_bytes;
use super::logs::{LogLevel, LogLine};
use super::mods::{ModCategory, ModEntry};
use super::screenshots::Screenshot;
use super::settings::{LaunchMode, MINECRAFT_VERSIONS, Settings};
use super::worlds::{Difficulty, GameMode, WorldEntry};

/// The biomes generated art cycles through.
const BIOME_CYCLE: [Biome; 8] = [
    Biome::End,
    Biome::Forest,
    Biome::Cherry,
    Biome::Ocean,
    Biome::Desert,
    Biome::Snow,
    Biome::Nether,
    Biome::Mushroom,
];

/// The first seeded instance is the reference the reference images show. Matching
/// on its id keeps later seeds that reuse the name from inheriting its data.
const REFERENCE_ID: &str = "seed-000";

fn is_reference(instance: &Instance) -> bool {
    instance.id == REFERENCE_ID
}

// Mods ----------------------------------------------------------------------

/// The reference's eight mods, in the order its list shows them.
const REFERENCE_MODS: [(&str, &str, &str, ModCategory, bool); 8] = [
    ("Sodium", "JellySquid", "0.5.8", ModCategory::Performance, true),
    ("Iris Shaders", "coderbot", "1.7.1", ModCategory::Graphics, true),
    ("Lithium", "CaffeineMC", "0.12.1", ModCategory::Performance, true),
    ("Fabric API", "FabricMC", "0.92.0", ModCategory::Library, true),
    ("Xaero's Minimap", "xaero96", "24.1.0", ModCategory::Hud, true),
    ("JEI", "mezz", "15.2.0", ModCategory::Utility, true),
    ("Mod Menu", "Prospector", "7.2.2", ModCategory::Utility, false),
    ("Continuity", "PepperCode1", "3.0.0", ModCategory::Graphics, true),
];

/// Names generated rows are drawn from; a second pass adds a round number.
const MOD_POOL: [(&str, &str, ModCategory); 12] = [
    ("Journey Map", "TeamJM", ModCategory::Utility),
    ("Waystones", "BlayTheNinth", ModCategory::Utility),
    ("Farmer's Delight", "vectorwing", ModCategory::Utility),
    ("Biomes O' Plenty", "Glitchfiend", ModCategory::Graphics),
    ("Dynamic Lights", "LambdaDev", ModCategory::Graphics),
    ("Starlight", "spinyfin", ModCategory::Performance),
    ("ModernFix", "embeddedt", ModCategory::Performance),
    ("Cloth Config", "shedaniel", ModCategory::Library),
    ("Jade", "Snownee", ModCategory::Hud),
    ("AppleSkin", "squeek502", ModCategory::Hud),
    ("Inventory Profiles Next", "Ciconia", ModCategory::Utility),
    ("Create", "simibubi", ModCategory::Library),
];

/// The installed mods: the reference rows for Aether Legacy, then generated
/// rows up to the instance's mod count.
pub fn mods(instance: &Instance) -> Vec<ModEntry> {
    let mut mods: Vec<ModEntry> = if is_reference(instance) {
        REFERENCE_MODS
            .iter()
            .map(|&(name, author, version, category, enabled)| ModEntry {
                name: name.to_string(),
                author: author.to_string(),
                version: version.to_string(),
                category,
                enabled,
            })
            .collect()
    } else {
        Vec::new()
    };

    let total = instance.mod_count as usize;
    mods.truncate(total);
    for index in mods.len()..total {
        mods.push(generated_mod(index));
    }
    mods
}

fn generated_mod(index: usize) -> ModEntry {
    let (name, author, category) = MOD_POOL[index % MOD_POOL.len()];
    let round = index / MOD_POOL.len();
    ModEntry {
        name: if round == 0 { name.to_string() } else { format!("{name} {}", round + 1) },
        author: author.to_string(),
        version: format!("{}.{}.{}", 1 + round % 4, index % 9, index % 7),
        category,
        enabled: index % 6 != 5,
    }
}

// Worlds --------------------------------------------------------------------

/// When a world was saved, relative to the moment the tab opens.
#[derive(Debug, Clone, Copy)]
enum Saved {
    MinutesAgo(i64),
    DaysAgo(i64),
    /// A fixed calendar day, for worlds saved long ago.
    On(i32, u32, u32),
}

/// A reference world before it is resolved against the current time.
struct WorldSeed {
    name: &'static str,
    game_mode: GameMode,
    difficulty: Difficulty,
    seed: i64,
    play_seconds: u32,
    size_bytes: u64,
    saved: Saved,
    version: &'static str,
    pinned: bool,
    synced: bool,
    protected: bool,
    biome: Biome,
}

/// The six worlds the reference shows, most recently saved first.
const REFERENCE_WORLDS: [WorldSeed; 6] = [
    WorldSeed {
        name: "Aether Hub",
        game_mode: GameMode::Survival,
        difficulty: Difficulty::Hard,
        seed: -4_827_193_047_281,
        play_seconds: 48 * 3600 + 12 * 60,
        size_bytes: 1_503_238_554,
        saved: Saved::MinutesAgo(40),
        version: "1.20.1",
        pinned: true,
        synced: true,
        protected: false,
        biome: Biome::End,
    },
    WorldSeed {
        name: "Creative Sandbox",
        game_mode: GameMode::Creative,
        difficulty: Difficulty::Peaceful,
        seed: 8_821_094_572,
        play_seconds: 9 * 3600 + 4 * 60,
        size_bytes: 432_013_312,
        saved: Saved::DaysAgo(1),
        version: "1.20.1",
        pinned: false,
        synced: true,
        protected: false,
        biome: Biome::Cherry,
    },
    WorldSeed {
        name: "SkyBlock Classic",
        game_mode: GameMode::Survival,
        difficulty: Difficulty::Normal,
        seed: 12_345,
        play_seconds: 22 * 3600 + 47 * 60,
        size_bytes: 297_795_584,
        saved: Saved::DaysAgo(3),
        version: "1.20.1",
        pinned: false,
        synced: false,
        protected: true,
        biome: Biome::Ocean,
    },
    WorldSeed {
        name: "Nether Outpost",
        game_mode: GameMode::Hardcore,
        difficulty: Difficulty::Hard,
        seed: -918_273_645,
        play_seconds: 104 * 3600 + 31 * 60,
        size_bytes: 2_254_857_830,
        saved: Saved::DaysAgo(9),
        version: "1.20.1",
        pinned: true,
        synced: false,
        protected: false,
        biome: Biome::Nether,
    },
    WorldSeed {
        name: "Pixel Town",
        game_mode: GameMode::Survival,
        difficulty: Difficulty::Normal,
        seed: 7_391_047,
        play_seconds: 31 * 3600 + 18 * 60,
        size_bytes: 939_524_096,
        saved: Saved::DaysAgo(15),
        version: "1.19.4",
        pinned: false,
        synced: false,
        protected: false,
        biome: Biome::Forest,
    },
    WorldSeed {
        name: "Desert Expedition",
        game_mode: GameMode::Adventure,
        difficulty: Difficulty::Hard,
        seed: -2_837_461,
        play_seconds: 14 * 3600 + 22 * 60,
        size_bytes: 675_282_944,
        saved: Saved::On(2026, 4, 8),
        version: "1.20.1",
        pinned: false,
        synced: false,
        protected: true,
        biome: Biome::Desert,
    },
];

/// Names generated worlds are drawn from; a second pass adds a round number.
const WORLD_POOL: [&str; 8] = [
    "Sky Island",
    "Frost Keep",
    "Ember Hollow",
    "Mossy Vale",
    "Crystal Reach",
    "Dune Camp",
    "Reef Haven",
    "Pine Lodge",
];

/// The saved worlds: the reference's six for Aether Legacy, otherwise a few
/// generated ones. Save times are resolved against `now`.
pub fn worlds(instance: &Instance, now: DateTime<Local>) -> Vec<WorldEntry> {
    if is_reference(instance) {
        REFERENCE_WORLDS.iter().map(|world| resolve(world, now)).collect()
    } else {
        let count = 2 + instance.art_seed as usize % 5;
        (0..count).map(|index| generated_world(index, now)).collect()
    }
}

fn resolve(world: &WorldSeed, now: DateTime<Local>) -> WorldEntry {
    let saved = match world.saved {
        Saved::MinutesAgo(minutes) => now - Duration::minutes(minutes),
        Saved::DaysAgo(days) => now - Duration::days(days),
        Saved::On(year, month, day) => Local.with_ymd_and_hms(year, month, day, 20, 0, 0).single().unwrap_or(now),
    };
    WorldEntry {
        name: world.name.to_string(),
        game_mode: world.game_mode,
        difficulty: world.difficulty,
        seed: world.seed,
        play_seconds: world.play_seconds,
        size_bytes: world.size_bytes,
        saved,
        version: world.version.to_string(),
        pinned: world.pinned,
        synced: world.synced,
        protected: world.protected,
        backup: false,
        biome: world.biome,
        art_seed: world.seed.unsigned_abs(),
    }
}

fn generated_world(index: usize, now: DateTime<Local>) -> WorldEntry {
    let name = WORLD_POOL[index % WORLD_POOL.len()];
    let round = index / WORLD_POOL.len();
    let (game_mode, difficulty) = match index % 3 {
        0 => (GameMode::Survival, Difficulty::Normal),
        1 => (GameMode::Creative, Difficulty::Peaceful),
        _ => (GameMode::Survival, Difficulty::Hard),
    };
    WorldEntry {
        name: if round == 0 { name.to_string() } else { format!("{name} {}", round + 1) },
        game_mode,
        difficulty,
        seed: index as i64 * 7_919 - 40_000,
        play_seconds: (index as u32 * 5 + 3) * 3600 + 17 * 60,
        size_bytes: (120 + index as u64 * 53) * 1024 * 1024,
        saved: now - Duration::days(index as i64 * 3 + 2),
        version: "1.20.1".to_string(),
        pinned: index == 0,
        synced: index.is_multiple_of(2),
        protected: index % 3 == 2,
        backup: index % 4 == 3,
        biome: BIOME_CYCLE[index % BIOME_CYCLE.len()],
        art_seed: index as u64 * 13 + 5,
    }
}

// Screenshots ---------------------------------------------------------------

/// The biomes of the reference's nine captures, in order.
const REFERENCE_SHOT_BIOMES: [Biome; 9] = [
    Biome::End,
    Biome::Nether,
    Biome::Mushroom,
    Biome::Ocean,
    Biome::Cherry,
    Biome::Snow,
    Biome::Forest,
    Biome::Desert,
    Biome::End,
];

/// The captures: the reference's nine for Aether Legacy, otherwise a few
/// generated ones.
pub fn screenshots(instance: &Instance) -> Vec<Screenshot> {
    if is_reference(instance) {
        REFERENCE_SHOT_BIOMES
            .iter()
            .enumerate()
            .map(|(index, &biome)| Screenshot {
                file_name: format!("2026-04-{:02}_18.{:02}.png", 10 + index, index),
                size_label: format_bytes(2_202_009),
                biome,
                art_seed: index as u64,
            })
            .collect()
    } else {
        (0..3 + instance.art_seed as usize % 4)
            .map(|index| Screenshot {
                file_name: format!("2026-03-{:02}_20.{:02}.png", 20 + index, index),
                size_label: format_bytes(1_700_000 + index as u64 * 120_000),
                biome: BIOME_CYCLE[index % BIOME_CYCLE.len()],
                art_seed: index as u64 + 1,
            })
            .collect()
    }
}

// Logs ----------------------------------------------------------------------

/// The reference's start-up log, line by line.
const REFERENCE_LOG: [(&str, LogLevel, &str); 9] = [
    ("18:42:03", LogLevel::Info, "Loading Fabric Loader 0.15.7"),
    ("18:42:04", LogLevel::Info, "Found 87 mods to load"),
    ("18:42:04", LogLevel::Info, "Injecting mixin patches..."),
    ("18:42:05", LogLevel::Warn, "Mod `create` registered legacy recipe handler"),
    ("18:42:06", LogLevel::Info, "Built registry snapshot (3,214 entries)"),
    ("18:42:07", LogLevel::Info, "Loading world: Aether_Dimensions"),
    ("18:42:08", LogLevel::Info, "Chunks generated: 892 · Entities: 1,204"),
    ("18:42:09", LogLevel::Info, "Ready · 58 FPS · 2.1 GB used"),
    ("18:42:10", LogLevel::Info, "Awaiting player input"),
];

/// The log: the reference's start-up lines for Aether Legacy, otherwise a
/// short start-up sequence shaped by the instance's loader and mods.
pub fn log_lines(instance: &Instance) -> Vec<LogLine> {
    if is_reference(instance) {
        REFERENCE_LOG
            .iter()
            .map(|&(time, level, message)| line(time, level, message.to_string()))
            .collect()
    } else {
        let mut lines = vec![line("18:40:11", LogLevel::Info, format!("Starting Minecraft {}", instance.mc_version))];
        if instance.loader != Loader::Vanilla {
            let version = instance.loader_version.as_deref().unwrap_or_default();
            lines.push(line("18:40:12", LogLevel::Info, format!("Loading {} {version}", instance.loader.label())));
        }
        if instance.mod_count > 0 {
            lines.push(line("18:40:13", LogLevel::Info, format!("Found {} mods to load", instance.mod_count)));
        }
        if instance.art_seed.is_multiple_of(3) {
            lines.push(line("18:40:14", LogLevel::Error, "Missing pack.mcmeta in resource pack".to_string()));
        }
        if instance.art_seed.is_multiple_of(2) {
            lines.push(line("18:40:14", LogLevel::Debug, "Tick profiler armed".to_string()));
        }
        lines.push(line("18:40:15", LogLevel::Info, "Awaiting player input".to_string()));
        lines
    }
}

fn line(time: &str, level: LogLevel, message: String) -> LogLine {
    LogLine {
        time: time.to_string(),
        level,
        message,
    }
}

// Settings ------------------------------------------------------------------

/// The launch configuration an instance starts with. The reference's values
/// for Aether Legacy; every other instance follows its own version and loader.
pub fn settings(instance: &Instance) -> Settings {
    let reference = is_reference(instance);
    let loader_versions = if reference {
        vec!["0.14.21".to_string(), "0.15.7".to_string(), "0.15.11".to_string()]
    } else {
        vec![instance.loader_version.clone().unwrap_or_else(|| "—".to_string())]
    };

    Settings {
        java_runtime: 0,
        garbage_collector: 0,
        minecraft_version: MINECRAFT_VERSIONS.iter().position(|version| *version == instance.mc_version).unwrap_or(0),
        loader_versions,
        loader_version: 0,
        memory_gb: if reference { 6 } else { 4 },
        system_java: false,
        snapshots: false,
        launch_mode: LaunchMode::Windowed,
        width: "1920".to_string(),
        height: "1080".to_string(),
        hide_launcher: true,
        quit_after_exit: false,
        game_args: String::new(),
        environment_variables: if reference { 3 } else { 0 },
        verbose_logging: reference,
        skip_integrity: false,
        inherited: 4,
    }
}

#[cfg(test)]
#[path = "../../../../tests/app/pages/instance/demo.rs"]
mod tests;
