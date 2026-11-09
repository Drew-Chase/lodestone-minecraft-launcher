//! The side a mod jar runs on. sievemc reads the Fabric, Forge and NeoForge metadata. A jar it cannot place is read
//! again for `quilt.mod.json`, with the rule the Quilt loader applies to that file.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::Deserialize;
use sievemc::{ModDetection, Side};

/// The name of Quilt's metadata file in a jar.
const QUILT_MANIFEST: &str = "quilt.mod.json";

/// Detects the side of the mod jar at `path`. Never fails: a jar whose side cannot be read is returned unsure
/// (`side: None`), with the reason as its evidence.
pub fn detect(path: impl AsRef<Path>) -> ModDetection {
    let path = path.as_ref();
    let mut detection = sievemc::detect_mod(path);
    if detection.side.is_some() {
        return detection;
    }
    if let Some(quilt) = quilt(path) {
        detection.loader = Some("quilt".to_string());
        if detection.mod_id.is_none() {
            detection.mod_id = quilt.id;
        }
        detection.side = quilt.side;
        detection.evidence = quilt.evidence;
    }
    detection
}

/// What a jar's `quilt.mod.json` says about its side.
struct QuiltSide {
    /// The mod's id, from `quilt_loader.id`, when the file names one.
    id: Option<String>,
    side: Option<Side>,
    evidence: String,
}

/// The side in the jar's `quilt.mod.json`, or `None` when the jar has no such file. A file that cannot be read is
/// reported as unsure rather than skipped, so the review shows why the mod is unsure.
fn quilt(path: &Path) -> Option<QuiltSide> {
    let mut archive = zip::ZipArchive::new(File::open(path).ok()?).ok()?;
    let mut entry = archive.by_name(QUILT_MANIFEST).ok()?;
    let mut bytes = Vec::new();
    if entry.read_to_end(&mut bytes).is_err() {
        return Some(unreadable());
    }
    Some(parse(&bytes))
}

/// The reading for a `quilt.mod.json` that is not valid JSON, or has no `quilt_loader` object.
fn unreadable() -> QuiltSide {
    QuiltSide {
        id: None,
        side: None,
        evidence: format!("{QUILT_MANIFEST} → could not be read"),
    }
}

/// Reads the side the manifest names, as the Quilt loader reads it.
fn parse(bytes: &[u8]) -> QuiltSide {
    let Ok(manifest) = serde_json::from_slice::<Manifest>(bytes) else {
        return unreadable();
    };
    let id = Some(manifest.quilt_loader.id).filter(|id| !id.is_empty());
    let environment = manifest.minecraft.and_then(|minecraft| minecraft.environment);
    match side_of(environment.as_deref()) {
        Some(side) => QuiltSide {
            id,
            evidence: format!("{QUILT_MANIFEST} → {}", label(&side)),
            side: Some(side),
        },
        None => QuiltSide {
            id,
            side: None,
            evidence: format!("{QUILT_MANIFEST} → unknown environment {:?}", environment.unwrap_or_default()),
        },
    }
}

/// The side a `minecraft.environment` value names. A missing value, `""` and `"*"` run on both sides, as the loader
/// treats a `quilt.mod.json` that does not set one. `None` is a value the loader rejects.
fn side_of(environment: Option<&str>) -> Option<Side> {
    match environment.unwrap_or("") {
        "" | "*" => Some(Side::ClientAndServer),
        "client" => Some(Side::ClientOnly),
        "dedicated_server" => Some(Side::ServerOnly),
        _ => None,
    }
}

/// The side's word in evidence, as sievemc writes it.
fn label(side: &Side) -> &'static str {
    match side {
        Side::ClientOnly => "client",
        Side::ServerOnly => "server",
        Side::ClientAndServer => "both",
    }
}

/// The parts of `quilt.mod.json` the side needs. Every other field is ignored.
#[derive(Deserialize)]
struct Manifest {
    quilt_loader: Loader,
    #[serde(default)]
    minecraft: Option<Minecraft>,
}

#[derive(Deserialize)]
struct Loader {
    id: String,
}

#[derive(Deserialize)]
struct Minecraft {
    #[serde(default)]
    environment: Option<String>,
}

#[cfg(test)]
#[path = "../../tests/server_pack/side.rs"]
mod tests;
