//! The portable launcher. It is a copy of the running launcher with a Lodestone package after it, so the one
//! file carries the instance and boots into it. On Windows and Linux the package is appended to the copy,
//! followed by a footer. On macOS the copy sits in an app bundle beside the package, so the copy's code
//! signature never covers appended bytes.

use std::fs::File;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use zip::CompressionMethod;
use zip::System;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use crate::error::Result;
use crate::format::Package;
use crate::format::lodestone;
use crate::progress::Control;

/// Ends a launcher that carries a package. It is the last 16 bytes of the file, after the package's length.
pub const MAGIC: &[u8; 16] = b"LODESTONE-PKG-V1";
/// The footer after the package: its length as a little-endian `u64`, then [`MAGIC`].
const FOOTER_LEN: u64 = 8 + 16;
/// The package's file name inside a macOS app bundle's `Contents/Resources` folder.
pub const SIDECAR: &str = "payload.lodestone";
/// The launcher's file name inside a macOS app bundle's `Contents/MacOS` folder.
pub const EXECUTABLE: &str = "lodestone";

/// Where a launcher's package is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Located {
    /// Appended to the launcher's own file: `len` bytes starting at `offset`.
    Appended { offset: u64, len: u64 },
    /// Beside the launcher, in a macOS app bundle's `Contents/Resources` folder.
    Sidecar(PathBuf),
}

/// Finds the package a launcher carries, if it carries one. A launcher whose footer is missing or does not
/// fit the file has no package.
pub fn locate(executable: &Path) -> io::Result<Option<Located>> {
    let mut file = File::open(executable)?;
    let len = file.metadata()?.len();
    if len >= FOOTER_LEN {
        file.seek(SeekFrom::End(-(FOOTER_LEN as i64)))?;
        let mut footer = [0u8; FOOTER_LEN as usize];
        file.read_exact(&mut footer)?;
        if &footer[8..] == MAGIC {
            let package_len = u64::from_le_bytes(footer[..8].try_into().expect("the length is eight bytes"));
            if package_len <= len - FOOTER_LEN {
                let offset = len - FOOTER_LEN - package_len;
                return Ok(Some(Located::Appended { offset, len: package_len }));
            }
        }
    }
    Ok(sidecar(executable).filter(|path| path.is_file()).map(Located::Sidecar))
}

/// The sidecar package of a launcher in a macOS app bundle: `Contents/Resources/payload.lodestone`, beside
/// `Contents/MacOS/<launcher>`.
pub fn sidecar(executable: &Path) -> Option<PathBuf> {
    Some(executable.parent()?.parent()?.join("Resources").join(SIDECAR))
}

/// A reader over the package `located` names. An appended package is read from `executable`.
pub fn open(executable: &Path, located: &Located) -> io::Result<Box<dyn Read + Send>> {
    match located {
        Located::Appended { offset, len } => {
            let mut file = File::open(executable)?;
            file.seek(SeekFrom::Start(*offset))?;
            Ok(Box::new(file.take(*len)))
        }
        Located::Sidecar(path) => Ok(Box::new(File::open(path)?)),
    }
}

/// Writes a portable launcher to `target`: a copy of `executable`, then `package`, then the footer. The file is
/// made executable.
pub fn write_binary(executable: &Path, target: &Path, package: &Package<'_>, control: &mut Control<'_>) -> Result<()> {
    let mut file = BufWriter::new(File::create(target)?);
    io::copy(&mut File::open(executable)?, &mut file)?;
    let start = file.stream_position()?;
    let mut file = lodestone::write_to(file, package, control)?;
    let end = file.stream_position()?;
    file.write_all(&(end - start).to_le_bytes())?;
    file.write_all(MAGIC)?;
    file.flush()?;
    drop(file);
    make_executable(target)?;
    Ok(())
}

/// Writes a macOS portable launcher to `target`: a zip holding `<app_name>.app`. Inside it are the launcher's
/// copy in `Contents/MacOS`, the package in `Contents/Resources`, and an `Info.plist` that names the bundle
/// `bundle_id`.
pub fn write_app(executable: &Path, target: &Path, app_name: &str, bundle_id: &str, package: &Package<'_>, control: &mut Control<'_>) -> Result<()> {
    let contents = format!("{app_name}.app/Contents");
    let mut zip = ZipWriter::new(BufWriter::new(File::create(target)?));
    // Readers take the mode bits from a zip only when its host is Unix. zip picks MS-DOS on Windows, which would
    // drop the launcher's executable bit, so the bundle always names Unix as its host.
    let plain = SimpleFileOptions::default()
        .system(System::Unix)
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    let program = SimpleFileOptions::default()
        .system(System::Unix)
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o755);
    let stored = SimpleFileOptions::default()
        .system(System::Unix)
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o644);

    zip.start_file(format!("{contents}/Info.plist"), plain)?;
    zip.write_all(info_plist(app_name, bundle_id).as_bytes())?;
    zip.start_file(format!("{contents}/MacOS/{EXECUTABLE}"), program)?;
    io::copy(&mut File::open(executable)?, &mut zip)?;
    zip.start_file(format!("{contents}/Resources/{SIDECAR}"), stored)?;
    lodestone::write_to(&mut zip, package, control)?;
    let mut output = zip.finish()?;
    output.flush()?;
    Ok(())
}

/// The `Info.plist` of a portable launcher's app bundle.
fn info_plist(app_name: &str, bundle_id: &str) -> String {
    let name = escape(app_name);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>{name}</string>
    <key>CFBundleDisplayName</key>
    <string>{name}</string>
    <key>CFBundleExecutable</key>
    <string>{EXECUTABLE}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_id}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
"#
    )
}

/// Escapes the characters XML gives a meaning to.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(unix)]
fn make_executable(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
#[path = "../tests/portable.rs"]
mod tests;
