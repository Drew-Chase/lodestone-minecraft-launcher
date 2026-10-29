//! Per-OS "fake java" stub shared by the integration tests (DESIGN.md §9).
//!
//! No Java installation and no Minecraft assets are required: this module writes
//! a small script into a fresh temp directory and the launcher treats it as
//! `java_path`, so tests observe the real command line end-to-end through the
//! stub's echoed arguments.

#![allow(dead_code)] // each test binary links the whole module but uses part of it

use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Marker the echo stub writes to stderr, so tests can assert stderr events.
pub const STUB_STDERR_LINE: &str = "STUB_STDERR_LINE";
/// Marker the sleeping stub writes to stdout once it is up.
pub const STUB_STARTED_LINE: &str = "STUB_STARTED_LINE";

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// A fake `java` executable plus a valid working directory, both inside a fresh
/// temporary directory that is removed (best effort) when dropped.
pub struct StubJava {
    /// The fresh temp directory holding the stub script; all derived paths hang
    /// off it.
    pub dir: PathBuf,
    /// Hand to `Launcher::builder` as `java_path`.
    pub java_path: PathBuf,
    /// Hand to `Launcher::builder` as `working_directory`.
    pub working_directory: PathBuf,
}

impl StubJava {
    /// Writes a stub script with `body` into a fresh temp directory.
    fn write(body: &str) -> std::io::Result<Self> {
        let dir = std::env::temp_dir().join(format!(
            "lodestone-launch-test-{}-{}",
            process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir_all(&dir)?;
        let working_directory = dir.join("work");
        std::fs::create_dir_all(&working_directory)?;

        #[cfg(windows)]
        let java_path = {
            // Spawning a `.cmd` is handled by CreateProcess, which runs it
            // through cmd.exe; std applies cmd-safe argument quoting for it.
            let path = dir.join("stub.cmd");
            std::fs::write(&path, body)?;
            path
        };

        #[cfg(not(windows))]
        let java_path = {
            use std::os::unix::fs::PermissionsExt;
            let path = dir.join("stub.sh");
            std::fs::write(&path, body)?;
            let mut permissions = std::fs::metadata(&path)?.permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&path, permissions)?;
            path
        };

        Ok(Self {
            dir,
            java_path,
            working_directory,
        })
    }

    /// A stub that echoes its full argument list to stdout (one line) and writes
    /// [`STUB_STDERR_LINE`] to stderr, then exits immediately.
    pub fn echo() -> std::io::Result<Self> {
        #[cfg(windows)]
        let body = "@echo off\r\necho %*\r\necho STUB_STDERR_LINE>&2\r\n";
        #[cfg(not(windows))]
        let body = "#!/bin/sh\necho \"$@\"\necho STUB_STDERR_LINE >&2\n";
        Self::write(body)
    }

    /// A stub that announces itself on stdout and then stalls long enough for a
    /// test to kill it. The sleep child's streams are redirected away from the
    /// game's pipes, so killing the stub closes those pipes immediately instead
    /// of waiting out the sleep.
    pub fn sleeper() -> std::io::Result<Self> {
        #[cfg(windows)]
        let body = "@echo off\r\necho STUB_STARTED_LINE\r\nping -n 11 127.0.0.1 >nul 2>&1\r\n";
        #[cfg(not(windows))]
        let body = "#!/bin/sh\necho STUB_STARTED_LINE\nsleep 10 >/dev/null 2>&1\n";
        Self::write(body)
    }
}

impl Drop for StubJava {
    fn drop(&mut self) {
        // Best effort: on Windows a still-running grandchild can hold the
        // directory, in which case cleanup silently fails and the OS temp dir
        // wins.
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// A valid [`Launcher`](lodestone_launch::Launcher) around `stub` with one of
/// every optional field set, ready for further typed setters.
pub fn builder_for(stub: &StubJava) -> lodestone_launch::LauncherBuilder {
    lodestone_launch::Launcher::builder(&stub.java_path, &stub.working_directory)
        .library_directory(stub.dir.join("libraries"))
        .natives_directory(stub.dir.join("natives"))
        .assets_directory(stub.dir.join("assets"))
        .main_class("com.stub.Main")
        .classpath([stub.dir.join("one.jar"), stub.dir.join("two.jar")])
        .launcher_name("lodestone-test")
        .launcher_version("0.0.0-test")
}
