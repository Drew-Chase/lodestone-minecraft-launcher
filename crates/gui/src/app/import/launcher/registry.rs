//! Reads the install folders the Windows uninstall entries name. A launcher installed to another drive is found through its
//! entry, so the folder it lives in does not have to be a default one.

use std::path::PathBuf;

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};

/// The uninstall entries of the current user and of the machine, and of the 32-bit programs on a 64-bit machine.
const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
const UNINSTALL_32: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

/// The install folders of the uninstall entries whose display name is one of `names`. An entry that cannot be read is
/// skipped.
pub fn install_locations(names: &[&str]) -> Vec<PathBuf> {
    if names.is_empty() {
        return Vec::new();
    }
    let mut found = Vec::new();
    let roots = [
        (HKEY_CURRENT_USER, UNINSTALL),
        (HKEY_LOCAL_MACHINE, UNINSTALL),
        (HKEY_LOCAL_MACHINE, UNINSTALL_32),
    ];
    for (hive, path) in roots {
        let Ok(root) = RegKey::predef(hive).open_subkey_with_flags(path, KEY_READ) else {
            continue;
        };
        for key_name in root.enum_keys().flatten() {
            let Ok(entry) = root.open_subkey_with_flags(&key_name, KEY_READ) else {
                continue;
            };
            let Ok(display): Result<String, _> = entry.get_value("DisplayName") else {
                continue;
            };
            if !names.iter().any(|name| name.eq_ignore_ascii_case(display.trim())) {
                continue;
            }
            let Ok(location): Result<String, _> = entry.get_value("InstallLocation") else {
                continue;
            };
            let location = location.trim().trim_matches('"').trim();
            if !location.is_empty() {
                found.push(PathBuf::from(location));
            }
        }
    }
    found.sort();
    found.dedup();
    found
}
