//! The macOS app bundle, `Lodestone.app`, and the disk image that carries it. The bundle is plain files, so it is
//! written on every OS. Only the disk image needs `hdiutil`, which runs on macOS alone.

use std::fs;
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
use std::ffi::OsStr;
#[cfg(target_os = "macos")]
use std::process::Command;

/// The app's display name. The bundle folder is named after it.
const APP_NAME: &str = "Lodestone";
/// The launcher binary inside the bundle. It keeps the crate's name, as the Windows and Linux builds do.
const EXECUTABLE: &str = "lodestone_launcher";
/// The bundle identifier. It is a default: change it if the project takes another reverse-DNS name.
const BUNDLE_ID: &str = "io.github.drew-chase.lodestone";
/// The oldest macOS release the bundle declares it runs on.
const MINIMUM_SYSTEM: &str = "11.0";
/// The icon's name inside `Contents/Resources`, without its extension.
const ICON_NAME: &str = "AppIcon";
/// The folder that the disk image's contents are staged in, next to the bundle. It is removed afterwards.
const STAGING_DIR: &str = "dmg-staging";

/// The usage line printed when the arguments are wrong.
pub const USAGE: &str = "usage: cargo xtask bundle-macos --binary <file> --version <X.Y.Z> --out <dir> \
     [--icon <file.icns>] [--dmg <file.dmg>]";

/// What `bundle-macos` builds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// The launcher binary to put in the bundle.
    pub binary: PathBuf,
    /// The `.icns` file that becomes the app icon.
    pub icon: PathBuf,
    /// The release version, `X.Y.Z`, written into the bundle's version fields.
    pub version: String,
    /// The folder the bundle is written to.
    pub out: PathBuf,
    /// The disk image to make from the bundle, when one is asked for.
    pub dmg: Option<PathBuf>,
}

/// Reads the options that follow `bundle-macos` on the command line.
pub fn options(args: &[String]) -> Result<Options, String> {
    let version = required(args, "--version")?;
    if !is_version(&version) {
        return Err(format!("--version must be X.Y.Z, not {version:?}"));
    }
    Ok(Options {
        binary: PathBuf::from(required(args, "--binary")?),
        icon: PathBuf::from(value(args, "--icon").unwrap_or("res/icon/icon.icns")),
        version,
        out: PathBuf::from(required(args, "--out")?),
        dmg: value(args, "--dmg").map(PathBuf::from),
    })
}

/// The value after `flag`, when the flag is there with one.
fn value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    let index = args.iter().position(|arg| arg == flag)?;
    args.get(index + 1).map(String::as_str)
}

fn required(args: &[String], flag: &str) -> Result<String, String> {
    value(args, flag).map(str::to_owned).ok_or_else(|| format!("missing {flag}\n{USAGE}"))
}

/// Whether `text` is three dot-separated numbers, such as `0.1.0`. The bundle's version fields take nothing else.
pub fn is_version(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

/// The bundle's `Info.plist` for `version`.
pub fn info_plist(version: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>{APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>{APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>{BUNDLE_ID}</string>
    <key>CFBundleExecutable</key>
    <string>{EXECUTABLE}</string>
    <key>CFBundleIconFile</key>
    <string>{ICON_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>CFBundleVersion</key>
    <string>{version}</string>
    <key>LSMinimumSystemVersion</key>
    <string>{MINIMUM_SYSTEM}</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.games</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
</dict>
</plist>
"#
    )
}

/// Writes `Lodestone.app` into `options.out` and returns its path. A bundle that is already there is replaced.
pub fn write_bundle(options: &Options) -> Result<PathBuf, String> {
    fs::create_dir_all(&options.out).map_err(|error| io_error(&options.out, error))?;
    let app = options.out.join(format!("{APP_NAME}.app"));
    if app.exists() {
        fs::remove_dir_all(&app).map_err(|error| io_error(&app, error))?;
    }
    let contents = app.join("Contents");
    let macos = contents.join("MacOS");
    let resources = contents.join("Resources");
    fs::create_dir_all(&macos).map_err(|error| io_error(&macos, error))?;
    fs::create_dir_all(&resources).map_err(|error| io_error(&resources, error))?;

    let executable = macos.join(EXECUTABLE);
    fs::copy(&options.binary, &executable).map_err(|error| copy_error(&options.binary, &executable, error))?;
    make_executable(&executable)?;
    let icon = resources.join(format!("{ICON_NAME}.icns"));
    fs::copy(&options.icon, &icon).map_err(|error| copy_error(&options.icon, &icon, error))?;
    let plist = contents.join("Info.plist");
    fs::write(&plist, info_plist(&options.version)).map_err(|error| io_error(&plist, error))?;
    Ok(app)
}

/// Builds the bundle, and the disk image as well when `options.dmg` asks for one. Returns the bundle's path.
pub fn bundle(options: &Options) -> Result<PathBuf, String> {
    let app = write_bundle(options)?;
    if let Some(dmg) = &options.dmg {
        if let Some(parent) = dmg.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|error| io_error(parent, error))?;
        }
        make_dmg(&app, &options.out.join(STAGING_DIR), dmg)?;
    }
    Ok(app)
}

/// Writes the disk image `dmg` from the bundle at `app`, with an Applications link beside it so the app can be
/// dragged across. The contents are staged in `staging`, which is removed afterwards.
#[cfg(target_os = "macos")]
pub fn make_dmg(app: &Path, staging: &Path, dmg: &Path) -> Result<(), String> {
    if staging.exists() {
        fs::remove_dir_all(staging).map_err(|error| io_error(staging, error))?;
    }
    fs::create_dir_all(staging).map_err(|error| io_error(staging, error))?;
    let staged_app = staging.join(format!("{APP_NAME}.app"));
    run("ditto", &[app.as_os_str(), staged_app.as_os_str()])?;
    std::os::unix::fs::symlink("/Applications", staging.join("Applications")).map_err(|error| io_error(staging, error))?;
    run(
        "hdiutil",
        &[
            OsStr::new("create"),
            OsStr::new("-volname"),
            OsStr::new(APP_NAME),
            OsStr::new("-srcfolder"),
            staging.as_os_str(),
            OsStr::new("-ov"),
            OsStr::new("-format"),
            OsStr::new("UDZO"),
            dmg.as_os_str(),
        ],
    )?;
    fs::remove_dir_all(staging).map_err(|error| io_error(staging, error))
}

/// The disk image needs `hdiutil`, which only macOS has.
#[cfg(not(target_os = "macos"))]
pub fn make_dmg(_app: &Path, _staging: &Path, _dmg: &Path) -> Result<(), String> {
    Err("the disk image needs macOS (hdiutil); run bundle-macos with --dmg on a Mac".to_string())
}

#[cfg(target_os = "macos")]
fn run(program: &str, args: &[&OsStr]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|error| format!("could not start {program}: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} failed ({status})"))
    }
}

fn io_error(path: &Path, error: std::io::Error) -> String {
    format!("{}: {error}", path.display())
}

fn copy_error(from: &Path, to: &Path, error: std::io::Error) -> String {
    format!("could not copy {} to {}: {error}", from.display(), to.display())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).map_err(|error| io_error(path, error))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
#[path = "../tests/macos.rs"]
mod tests;
