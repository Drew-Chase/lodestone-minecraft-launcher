//! The saved worlds in `saves/`. A save is a folder that holds a `level.dat`.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Local, Utc};
use flate2::read::GzDecoder;
use serde::Deserialize;

use super::super::worlds::{Difficulty, GameMode};
use super::{decode_image, dir_size};

/// World ticks per second. `Time` in `level.dat` counts ticks.
const TICKS_PER_SECOND: u64 = 20;

/// One saved world.
#[derive(Debug, Clone)]
pub struct WorldFile {
    /// The save's folder, which holds `level.dat`.
    pub path: PathBuf,
    /// The name `level.dat` gives the world, or the folder name without one.
    pub name: String,
    pub game_mode: Option<GameMode>,
    pub difficulty: Option<Difficulty>,
    pub seed: Option<i64>,
    /// The Minecraft version that last saved the world.
    pub version: Option<String>,
    /// The folder's total size on disk.
    pub size_bytes: u64,
    /// When the world was last played: `LastPlayed` from `level.dat`, or when the file was last written.
    pub saved: Option<DateTime<Local>>,
    /// The time spent in the world, from `Time` in `level.dat`.
    pub play_time: Option<Duration>,
    /// The save's `icon.png`, when it has one.
    pub icon: Option<iced::widget::image::Handle>,
}

/// Every save in `dir`, most recently saved first.
pub fn scan(dir: &Path) -> Vec<WorldFile> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut worlds: Vec<WorldFile> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| read_world(&entry.path()))
        .collect();
    worlds.sort_by_key(|world| std::cmp::Reverse(world.saved));
    worlds
}

/// The save in `path`, or `None` when the folder has no `level.dat`.
pub fn read_world(path: &Path) -> Option<WorldFile> {
    let level_path = path.join("level.dat");
    let written = std::fs::metadata(&level_path).ok()?.modified().ok().map(DateTime::<Local>::from);
    let folder = path.file_name()?.to_string_lossy().into_owned();
    // A `level.dat` that will not decode still lists the save, with the folder
    // name and no details.
    let level = read_level(&level_path).unwrap_or_default();

    let game_mode = match (level.hardcore.unwrap_or(0) != 0, level.game_type) {
        (true, _) => Some(GameMode::Hardcore),
        (false, Some(0)) => Some(GameMode::Survival),
        (false, Some(1)) => Some(GameMode::Creative),
        (false, Some(2)) => Some(GameMode::Adventure),
        _ => None,
    };
    let difficulty = match level.difficulty {
        Some(0) => Some(Difficulty::Peaceful),
        Some(1) => Some(Difficulty::Easy),
        Some(2) => Some(Difficulty::Normal),
        Some(3) => Some(Difficulty::Hard),
        _ => None,
    };
    // `LastPlayed` is epoch milliseconds. A zero or negative value is not a time, so the file's last
    // write stands in for it.
    let saved = level
        .last_played
        .filter(|millis| *millis > 0)
        .and_then(DateTime::<Utc>::from_timestamp_millis)
        .map(|played| played.with_timezone(&Local))
        .or(written);
    let play_time = level
        .time
        .and_then(|ticks| u64::try_from(ticks).ok())
        .map(|ticks| Duration::from_secs(ticks / TICKS_PER_SECOND));

    Some(WorldFile {
        path: path.to_path_buf(),
        name: level.level_name.filter(|name| !name.trim().is_empty()).unwrap_or(folder),
        game_mode,
        difficulty,
        seed: level.world_gen_settings.and_then(|settings| settings.seed).or(level.random_seed),
        version: level.version.and_then(|version| version.name),
        size_bytes: dir_size(path),
        saved,
        play_time,
        icon: decode_image(&path.join("icon.png"), None),
    })
}

/// Decodes a `level.dat`, which is gzip-compressed NBT. `None` when the file is
/// missing or does not decode.
fn read_level(path: &Path) -> Option<LevelData> {
    let file = File::open(path).ok()?;
    let level: LevelFile = fastnbt::from_reader(GzDecoder::new(BufReader::new(file))).ok()?;
    Some(level.data)
}

/// The top of `level.dat`: every key sits under `Data`.
#[derive(Deserialize)]
struct LevelFile {
    #[serde(rename = "Data")]
    data: LevelData,
}

/// The `level.dat` keys the Worlds tab shows. Each is optional, because older
/// saves and saves written by other tools leave some of them out.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct LevelData {
    level_name: Option<String>,
    /// 0 survival, 1 creative, 2 adventure, 3 spectator.
    game_type: Option<i32>,
    /// A byte flag; it is written in lowercase.
    #[serde(rename = "hardcore")]
    hardcore: Option<i8>,
    /// 0 peaceful, 1 easy, 2 normal, 3 hard.
    difficulty: Option<i8>,
    version: Option<GameVersion>,
    world_gen_settings: Option<WorldGenSettings>,
    /// Saves from before 1.16 keep the seed here.
    random_seed: Option<i64>,
    /// Epoch milliseconds of the last time the save was played.
    last_played: Option<i64>,
    /// Ticks the world has existed, 20 to the second.
    time: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct GameVersion {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct WorldGenSettings {
    seed: Option<i64>,
}

#[cfg(test)]
#[path = "../../../../../tests/app/pages/instance/files/worlds.rs"]
mod tests;
