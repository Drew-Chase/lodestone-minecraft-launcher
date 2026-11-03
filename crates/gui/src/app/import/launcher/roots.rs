//! The folders each launcher installs to, or keeps its data in, by default. A folder listed here is only a candidate: the
//! launcher counts as found when the folder holds its instances.

use std::path::PathBuf;

use super::Launcher;

/// The folders `launcher` is looked for in by default, most likely first.
pub(super) fn default_folders(launcher: Launcher) -> Vec<PathBuf> {
    let data = dirs::data_dir();
    let local = dirs::data_local_dir();
    let home = dirs::home_dir();
    let mut folders = Vec::new();
    let mut add = |base: &Option<PathBuf>, parts: &[&str]| {
        if let Some(base) = base {
            folders.push(parts.iter().fold(base.clone(), |path, part| path.join(part)));
        }
    };
    match launcher {
        Launcher::Prism => {
            add(&data, &["PrismLauncher"]);
            add(&home, &[".var", "app", "org.prismlauncher.PrismLauncher", "data", "PrismLauncher"]);
            add(&home, &["scoop", "persist", "prismlauncher"]);
        }
        // MultiMC is extracted wherever it is kept, so only a located folder finds it.
        Launcher::MultiMc => {}
        Launcher::AtLauncher => {
            add(&data, &["ATLauncher"]);
            add(&local, &["ATLauncher"]);
            if let Some(programs) = std::env::var_os("ProgramFiles") {
                folders.push(PathBuf::from(programs).join("ATLauncher"));
            }
        }
        Launcher::Minecraft => {
            #[cfg(windows)]
            {
                add(&data, &[".minecraft"]);
                add(
                    &local,
                    &["Packages", "Microsoft.4297127D64EC6_8wekyb3d8bbwe", "LocalCache", "Roaming", ".minecraft"],
                );
            }
            #[cfg(target_os = "macos")]
            add(&data, &["minecraft"]);
            #[cfg(target_os = "linux")]
            add(&home, &[".minecraft"]);
        }
        Launcher::CurseForge => {
            add(&home, &["curseforge", "minecraft"]);
            add(&home, &["Documents", "Curseforge", "Minecraft"]);
        }
        Launcher::Modrinth => {
            add(&data, &["ModrinthApp"]);
            add(&data, &["com.modrinth.theseus"]);
            add(&local, &["ModrinthApp"]);
        }
    }
    folders
}
