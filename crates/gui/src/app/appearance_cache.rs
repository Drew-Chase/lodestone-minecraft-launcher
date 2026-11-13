//! The last appearance of each Microsoft account, kept on disk. The launcher starts before the sign-in it restores in the
//! background has read the profile again, so the face and the Skins page start from this copy, and the fetch that follows
//! the sign-in replaces it. Each account's files sit in `<config dir>/appearance/<uuid>/`. Every call reads or writes the
//! disk, so callers run them on the blocking pool.

use std::path::PathBuf;

use crate::app::pages::skins::Appearance;
use crate::app::paths;

const SKIN: &str = "skin.png";
const CAPE: &str = "cape.png";
/// Present when the skin uses the slim model.
const SLIM: &str = "slim";

/// The folder that holds every account's copy.
pub fn root() -> Option<PathBuf> {
    Some(paths::config_dir()?.join("appearance"))
}

/// The folder of the account `uuid`. A uuid is hex with optional dashes, and anything else names no folder, so a profile id
/// can never point outside the cache.
fn folder(uuid: &str) -> Option<PathBuf> {
    if uuid.is_empty() || !uuid.chars().all(|character| character.is_ascii_hexdigit() || character == '-') {
        return None;
    }
    Some(root()?.join(uuid))
}

/// The appearance kept for the account `uuid`, when there is one.
pub fn load(uuid: &str) -> Option<Appearance> {
    let folder = folder(uuid)?;
    let skin_png = std::fs::read(folder.join(SKIN)).ok()?;
    let cape_png = std::fs::read(folder.join(CAPE)).ok();
    let slim = folder.join(SLIM).is_file();
    Some(Appearance { skin_png, cape_png, slim })
}

/// Keeps `appearance` as the account `uuid`'s copy. A cape or slim marker the appearance no longer has is removed.
pub fn store(uuid: &str, appearance: &Appearance) -> std::io::Result<()> {
    let Some(folder) = folder(uuid) else {
        return Ok(());
    };
    std::fs::create_dir_all(&folder)?;
    lodestone_fs::write_atomic(&folder.join(SKIN), &appearance.skin_png)?;
    match &appearance.cape_png {
        Some(cape) => lodestone_fs::write_atomic(&folder.join(CAPE), cape)?,
        None => remove_if_present(folder.join(CAPE))?,
    }
    if appearance.slim {
        lodestone_fs::write_atomic(&folder.join(SLIM), b"")?;
    } else {
        remove_if_present(folder.join(SLIM))?;
    }
    Ok(())
}

/// Removes every account's copy.
pub fn clear() -> std::io::Result<()> {
    match root() {
        Some(root) => match std::fs::remove_dir_all(root) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        },
        None => Ok(()),
    }
}

fn remove_if_present(path: PathBuf) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}
