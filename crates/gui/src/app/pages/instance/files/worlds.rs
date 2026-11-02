//! The saved worlds in `saves/`. A save is a folder that holds a `level.dat`.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Local, Utc};
use flate2::read::GzDecoder;
use iced::widget::image::Handle;
use serde::Deserialize;

use super::{decode_world_art, dir_size};
use crate::app::lang;

/// World ticks per second. `Time` in `level.dat` counts ticks.
const TICKS_PER_SECOND: u64 = 20;

/// The game mode a world was created in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Survival,
    Creative,
    Adventure,
    Hardcore,
}

impl GameMode {
    pub fn label(self) -> &'static str {
        match self {
            GameMode::Survival => lang::t("instance.worlds.mode.survival"),
            GameMode::Creative => lang::t("instance.worlds.mode.creative"),
            GameMode::Adventure => lang::t("instance.worlds.mode.adventure"),
            GameMode::Hardcore => lang::t("instance.worlds.mode.hardcore"),
        }
    }
}

/// The difficulty a world was set to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Peaceful => lang::t("instance.worlds.difficulty.peaceful"),
            Difficulty::Easy => lang::t("instance.worlds.difficulty.easy"),
            Difficulty::Normal => lang::t("instance.worlds.difficulty.normal"),
            Difficulty::Hard => lang::t("instance.worlds.difficulty.hard"),
        }
    }
}

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
    pub icon: Option<Handle>,
    /// The icon blurred into the card's backdrop, when the save has an icon.
    pub backdrop: Option<Handle>,
    /// Whether the instance's `backups/` folder holds a zip of this save.
    pub backed_up: bool,
}

impl WorldFile {
    /// The save's folder name, which the pins and the backups name it by.
    pub fn folder(&self) -> String {
        self.path.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned())
    }

    /// Whether the world's name or its seed contains `query`, which is lower case. An empty query matches every world.
    pub fn matches(&self, query: &str) -> bool {
        query.is_empty() || self.name.to_lowercase().contains(query) || self.seed.is_some_and(|seed| seed.to_string().contains(query))
    }
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
    let backed_up = has_backup(path, &folder);
    let art = decode_world_art(&path.join("icon.png"));

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
        icon: art.as_ref().map(|(icon, _)| icon.clone()),
        backdrop: art.map(|(_, backdrop)| backdrop),
        backed_up,
    })
}

/// Whether the instance folder that holds the save `path` has a zip of that save in `backups/`.
fn has_backup(path: &Path, folder: &str) -> bool {
    let Some(instance) = path.parent().and_then(Path::parent) else {
        return false;
    };
    let Ok(entries) = std::fs::read_dir(instance.join("backups")) else {
        return false;
    };
    entries
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_str().is_some_and(|name| is_backup_of(name, folder)))
}

/// Whether `name` is a zip that `back_up` wrote for the save `folder`: `<folder>-<stamp>.zip`.
fn is_backup_of(name: &str, folder: &str) -> bool {
    name.strip_prefix(folder)
        .and_then(|rest| rest.strip_prefix('-'))
        .and_then(|rest| rest.strip_suffix(".zip"))
        .is_some_and(looks_like_stamp)
}

/// Whether `stamp` has the `%Y-%m-%d_%H.%M.%S` shape that `back_up` writes. The shape keeps a world named
/// "Hub" from matching a backup of "Hub-2020" and its own folder.
fn looks_like_stamp(stamp: &str) -> bool {
    const SHAPE: &[u8] = b"dddd-dd-dd_dd.dd.dd";
    stamp.len() == SHAPE.len()
        && stamp.bytes().zip(SHAPE).all(|(byte, shape)| match shape {
            b'd' => byte.is_ascii_digit(),
            separator => byte == *separator,
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
