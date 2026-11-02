//! Starting the installer. A launcher from the Windows installer runs the downloaded installer elevated and silent.

use std::path::Path;

use crate::error::Error;

/// Whether the launcher at `exe` came from the Windows installer. The installer writes `uninstall.exe` beside the
/// launcher, and a portable copy has no such file.
pub fn installed_with_installer(exe: &Path) -> bool {
    exe.parent().is_some_and(|dir| dir.join("uninstall.exe").is_file())
}

/// Runs the installer with `/S`, which installs without a window. Windows asks for administrator rights first, so
/// the call returns once the prompt is answered and the installer has started, or once the prompt is declined.
#[cfg(windows)]
pub fn run_silent_elevated(installer: &Path) -> Result<(), Error> {
    use std::ffi::OsStr;
    use std::ptr::null;
    use std::ptr::null_mut;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

    let verb = wide(OsStr::new("runas"));
    let file = wide(installer.as_os_str());
    let parameters = wide(OsStr::new("/S"));
    // SAFETY: each string is a NUL-terminated UTF-16 buffer that lives until the call returns. The window handle and
    // the working directory are optional, so they are null.
    let result = unsafe { ShellExecuteW(null_mut(), verb.as_ptr(), file.as_ptr(), parameters.as_ptr(), null(), SW_HIDE) };
    // ShellExecute reports success with a value above 32. Anything at or below 32 is an error code.
    let code = result as isize;
    if code > 32 {
        Ok(())
    } else {
        Err(Error::Launch(format!("Windows did not start the installer (code {code})")))
    }
}

/// Only Windows has the installer, so the other systems never reach it.
#[cfg(not(windows))]
pub fn run_silent_elevated(_installer: &Path) -> Result<(), Error> {
    Err(Error::Launch("the Windows installer runs on Windows only".to_string()))
}

/// `text` as a NUL-terminated UTF-16 string, the form the Windows calls take.
#[cfg(windows)]
fn wide(text: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    text.encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
#[path = "../tests/install.rs"]
mod tests;
