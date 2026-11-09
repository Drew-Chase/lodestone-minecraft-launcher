//! Reads the mods of an instance, and lists the top-level folders of its folder, on the blocking pool. A scan streams a
//! tick for each jar, and stops at the next jar once its cancel flag is set.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use base64::Engine as _;
use iced::widget::image;
use lodestone_export::plan::is_leftover;
use lodestone_export::server_pack::plan::MODS;
use lodestone_export::server_pack::{ModDetection, side};
use sipper::{Sender as ProgressSender, Sipper, sipper};
use tokio::sync::mpsc;

use crate::app::pages::server_pack::{FolderRow, ModRow, ScanTick};

/// The error a scan stops with when its cancel flag is set.
pub const CANCELLED: &str = "cancelled";

/// Reads every mod jar in the `mods` folder of the instance folder `folder`, sorted by file name. A folder with no
/// `mods` folder has no mods. The result is the mods read, or the error the scan stopped at.
pub fn run(folder: PathBuf, cancel: Arc<AtomicBool>) -> impl Sipper<Result<Vec<ModRow>, String>, ScanTick> {
    sipper(move |mut progress: ProgressSender<ScanTick>| async move {
        let (reports, mut received) = mpsc::unbounded_channel::<ScanTick>();
        let work_folder = folder.clone();
        let handle = tokio::task::spawn_blocking(move || {
            let files = jars(&work_folder.join(MODS));
            let total = files.len();
            let mut mods = Vec::with_capacity(total);
            for (index, jar) in files.iter().enumerate() {
                if cancel.load(Ordering::SeqCst) {
                    return Err(CANCELLED.to_string());
                }
                let detection = side::detect(jar);
                let _ = reports.send(ScanTick {
                    folder: work_folder.clone(),
                    index: index + 1,
                    total,
                    name: detection.filename.clone(),
                    side: detection.side.clone(),
                });
                let icon = icon_of(&detection);
                mods.push(ModRow { detection, icon });
            }
            Ok(mods)
        });
        while let Some(tick) = received.recv().await {
            progress.send(tick).await;
        }
        match handle.await {
            Ok(result) => result,
            Err(error) => Err(error.to_string()),
        }
    })
}

/// Lists the top-level entries of the instance folder `folder`, except its `mods` folder and the markers an install
/// leaves behind. Folders come first, then files, each sorted by name.
pub async fn list_folders(folder: PathBuf) -> Result<Vec<FolderRow>, String> {
    match tokio::task::spawn_blocking(move || read_folders(&folder)).await {
        Ok(result) => result,
        Err(error) => Err(error.to_string()),
    }
}

/// The top-level entries of `folder`, as the Folders tab shows them.
fn read_folders(folder: &Path) -> Result<Vec<FolderRow>, String> {
    let entries = std::fs::read_dir(folder).map_err(|error| error.to_string())?;
    let mut rows: Vec<FolderRow> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == MODS || is_leftover(&name) {
                return None;
            }
            let file_type = entry.file_type().ok()?;
            if file_type.is_symlink() {
                return None;
            }
            Some(FolderRow {
                name,
                is_folder: file_type.is_dir(),
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        b.is_folder
            .cmp(&a.is_folder)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(rows)
}

/// The mod jars directly inside `mods`, sorted by name. A disabled jar (`*.jar.disabled`) is not one.
fn jars(mods: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(mods) else {
        return Vec::new();
    };
    let mut jars: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("jar")))
        .collect();
    jars.sort();
    jars
}

/// The icon a mod declares, decoded from the `data:` URI sievemc gives it. `None` when it declares none, or the URI is
/// not one this reads.
fn icon_of(detection: &ModDetection) -> Option<image::Handle> {
    let uri = detection.icon.as_deref()?;
    let (_, encoded) = uri.split_once(";base64,")?;
    let bytes = base64::engine::general_purpose::STANDARD.decode(encoded).ok()?;
    Some(image::Handle::from_bytes(bytes))
}
