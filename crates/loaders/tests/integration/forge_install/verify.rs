//! What a finished Forge install must leave behind, and the layout of its installer. The installer's own checks run first, so
//! these are the facts the launcher relies on afterwards: the version JSON the install wrote, and the Forge libraries it names.

use std::io::Read;
use std::path::Path;

use lodestone_loaders::InstalledVersion;
use serde_json::Value;

/// Checks the install of `minecraft` that `installed` names under `game_dir`. Returns the first problem found. A `legacy` install
/// (a `versionInfo` profile) keeps the profile's JSON as it is, and that JSON may leave out `inheritsFrom`.
pub(crate) fn installed(game_dir: &Path, minecraft: &str, installed: &InstalledVersion, legacy: bool) -> Result<(), String> {
    let json_path = game_dir.join("versions").join(&installed.id).join(format!("{}.json", installed.id));
    if installed.json != json_path {
        return Err(format!("the install names {}, not {}", installed.json.display(), json_path.display()));
    }
    let text = std::fs::read_to_string(&json_path).map_err(|error| format!("could not read {}: {error}", json_path.display()))?;
    let json: Value = serde_json::from_str(&text).map_err(|error| format!("{} is not JSON: {error}", json_path.display()))?;
    if json.get("id").and_then(Value::as_str) != Some(installed.id.as_str()) {
        return Err(format!("{} does not name itself {}", json_path.display(), installed.id));
    }
    let parent = json.get("inheritsFrom").and_then(Value::as_str);
    if parent != Some(minecraft) && !(legacy && parent.is_none()) {
        return Err(format!("{} does not inherit Minecraft {minecraft}", json_path.display()));
    }
    if json.get("mainClass").and_then(Value::as_str).is_none_or(str::is_empty) {
        return Err(format!("{} names no mainClass", json_path.display()));
    }
    let missing: Vec<String> = forge_libraries(&json)
        .into_iter()
        .filter(|path| !game_dir.join("libraries").join(path).is_file())
        .collect();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("Forge libraries the version names are missing: {}", missing.join(", ")))
    }
}

/// The maven paths, under `libraries/`, of the Forge libraries a version JSON names. The installer places these itself. The
/// other libraries are fetched when the game launches, so they are not checked here.
fn forge_libraries(json: &Value) -> Vec<String> {
    json.get("libraries")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|library| library.get("name").and_then(Value::as_str))
        .filter(|name| name.starts_with("net.minecraftforge:"))
        .filter_map(maven_path)
        .collect()
}

/// The path under `libraries/` of a maven coordinate `group:artifact:version[:classifier]`.
fn maven_path(coordinate: &str) -> Option<String> {
    let mut parts = coordinate.split(':');
    let (group, artifact, version) = (parts.next()?, parts.next()?, parts.next()?);
    let file = match parts.next() {
        Some(classifier) if !classifier.is_empty() => format!("{artifact}-{version}-{classifier}.jar"),
        _ => format!("{artifact}-{version}.jar"),
    };
    Some(format!("{}/{artifact}/{version}/{file}", group.replace('.', "/")))
}

/// The layout of an installer jar, for the report: what its `install_profile.json` holds, and whether it carries a
/// `version.json`. A profile with `processors` runs them (Forge 1.13 on), and one with only a `versionInfo` is the older form.
pub(crate) fn installer_layout(jar: &Path) -> String {
    let Ok(file) = std::fs::File::open(jar) else {
        return "unreadable".to_string();
    };
    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return "not a zip".to_string();
    };
    let profile = read_entry(&mut archive, "install_profile.json").and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let shape = match profile {
        Some(profile) if profile.get("processors").is_some() => "processors",
        Some(profile) if profile.get("versionInfo").is_some() => "versionInfo",
        Some(_) => "profile",
        None => "no profile",
    };
    let version = if archive.by_name("version.json").is_ok() {
        " + version.json"
    } else {
        ""
    };
    format!("{shape}{version}")
}

/// The text of one entry of an archive, when it is there and reads as text.
fn read_entry(archive: &mut zip::ZipArchive<std::fs::File>, name: &str) -> Option<String> {
    let mut entry = archive.by_name(name).ok()?;
    let mut text = String::new();
    entry.read_to_string(&mut text).ok()?;
    Some(text)
}
