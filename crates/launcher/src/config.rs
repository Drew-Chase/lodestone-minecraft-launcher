//! The validated, immutable launch description and its required-first builder.

use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::error::LaunchError;

/// A validated, immutable launch description of one Minecraft client process.
///
/// Construct through [`Launcher::builder`]; [`LauncherBuilder::build`] validates
/// eagerly so misconfiguration surfaces at build time with the offending path
/// attached, not at spawn time inside a game session.
///
/// `Launcher` is [`Clone`] + [`Send`] + [`Sync`], and [`Launcher::launch`] takes a
/// shared reference: launching the same configuration twice (two instances) is
/// legal and stateless. There are no globals, no statics, no interior mutability.
///
/// All argument lists are `Vec<String>` / `Vec<PathBuf>`; single-string argument
/// fields are forbidden because splitting a `String` on spaces breaks quoted args
/// and paths with spaces (`C:\Program Files\...`).
#[derive(Debug, Clone)]
pub struct Launcher {
    java_path: PathBuf,
    working_directory: PathBuf,
    library_directory: PathBuf,
    natives_directory: Option<PathBuf>,
    assets_directory: Option<PathBuf>,
    jvm_args: Vec<String>,
    main_class: String,
    classpath: Vec<PathBuf>,
    game_args: Vec<String>,
    env: BTreeMap<String, OsString>,
    launcher_name: String,
    launcher_version: String,
}

impl Launcher {
    /// Starts building a launch description around the two fields every launch
    /// needs. Everything else is an optional typed setter on [`LauncherBuilder`].
    #[must_use]
    pub fn builder(java_path: impl Into<PathBuf>, working_directory: impl Into<PathBuf>) -> LauncherBuilder {
        LauncherBuilder::new(java_path, working_directory)
    }

    /// The java executable to run. Always absolute, existing, and a file: no
    /// `PATH` lookup is performed.
    #[must_use]
    pub fn java_path(&self) -> &Path {
        &self.java_path
    }

    /// The directory the game runs in. Exists and is a directory; never created
    /// by this crate.
    #[must_use]
    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    /// The directory fed to `${library_directory}`. Empty (unset) until a value
    /// is provided.
    #[must_use]
    pub fn library_directory(&self) -> &Path {
        &self.library_directory
    }

    /// The directory driving `-Djava.library.path` and `${natives_directory}`.
    #[must_use]
    pub fn natives_directory(&self) -> Option<&Path> {
        self.natives_directory.as_deref()
    }

    /// The directory fed to `${assets_root}`.
    #[must_use]
    pub fn assets_directory(&self) -> Option<&Path> {
        self.assets_directory.as_deref()
    }

    /// JVM arguments, emitted in order before `-cp`.
    #[must_use]
    pub fn jvm_args(&self) -> &[String] {
        &self.jvm_args
    }

    /// The main class, emitted immediately after `-cp`. Empty when unset.
    #[must_use]
    pub fn main_class(&self) -> &str {
        &self.main_class
    }

    /// Classpath entries, joined via `std::env::join_paths` with the platform
    /// separator (`;` on Windows, `:` elsewhere).
    #[must_use]
    pub fn classpath(&self) -> &[PathBuf] {
        &self.classpath
    }

    /// Game arguments, emitted in order after the main class with environmental
    /// placeholders expanded.
    #[must_use]
    pub fn game_args(&self) -> &[String] {
        &self.game_args
    }

    /// Extra environment variables for the game process, ordered deterministically.
    #[must_use]
    pub fn env(&self) -> &BTreeMap<String, OsString> {
        &self.env
    }

    /// The name fed to `${launcher_name}`.
    #[must_use]
    pub fn launcher_name(&self) -> &str {
        &self.launcher_name
    }

    /// The version fed to `${launcher_version}`.
    #[must_use]
    pub fn launcher_version(&self) -> &str {
        &self.launcher_version
    }

    // `launch(&self)` lives in `process.rs`: it is the crate's one side-effectful
    // entry point and belongs beside the supervision code it drives.
}

/// Required-first builder for [`Launcher`].
///
/// The two genuinely required fields (java path and working directory) are
/// constructor arguments via [`Launcher::builder`]; everything else is an
/// optional typed setter. [`LauncherBuilder::build`] validates eagerly so
/// misconfiguration surfaces at build time with the offending path attached.
#[derive(Debug, Clone)]
pub struct LauncherBuilder {
    java_path: PathBuf,
    working_directory: PathBuf,
    library_directory: PathBuf,
    natives_directory: Option<PathBuf>,
    assets_directory: Option<PathBuf>,
    jvm_args: Vec<String>,
    main_class: String,
    classpath: Vec<PathBuf>,
    game_args: Vec<String>,
    env: BTreeMap<String, OsString>,
    launcher_name: String,
    launcher_version: String,
}

impl LauncherBuilder {
    /// Creates a builder around the two fields every launch needs.
    #[must_use]
    pub fn new(java_path: impl Into<PathBuf>, working_directory: impl Into<PathBuf>) -> Self {
        Self {
            java_path: java_path.into(),
            working_directory: working_directory.into(),
            library_directory: PathBuf::new(),
            natives_directory: None,
            assets_directory: None,
            jvm_args: Vec::new(),
            main_class: String::new(),
            classpath: Vec::new(),
            game_args: Vec::new(),
            env: BTreeMap::new(),
            launcher_name: String::new(),
            launcher_version: String::new(),
        }
    }

    /// Sets the directory fed to `${library_directory}`.
    #[must_use]
    pub fn library_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.library_directory = path.into();
        self
    }

    /// Sets the directory driving `-Djava.library.path` and `${natives_directory}`.
    #[must_use]
    pub fn natives_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.natives_directory = Some(path.into());
        self
    }

    /// Sets the directory fed to `${assets_root}`.
    #[must_use]
    pub fn assets_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.assets_directory = Some(path.into());
        self
    }

    /// Replaces the JVM arguments, emitted in order before `-cp`.
    #[must_use]
    pub fn jvm_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.jvm_args = args.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the main class, emitted immediately after `-cp`.
    #[must_use]
    pub fn main_class(mut self, main_class: impl Into<String>) -> Self {
        self.main_class = main_class.into();
        self
    }

    /// Replaces the classpath entries.
    #[must_use]
    pub fn classpath(mut self, entries: impl IntoIterator<Item = impl Into<PathBuf>>) -> Self {
        self.classpath = entries.into_iter().map(Into::into).collect();
        self
    }

    /// Replaces the game arguments; environmental placeholders are expanded at
    /// command-assembly time.
    #[must_use]
    pub fn game_args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.game_args = args.into_iter().map(Into::into).collect();
        self
    }

    /// Replaces the environment variables for the game process. The `BTreeMap`
    /// keeps the ordering deterministic.
    #[must_use]
    pub fn env(mut self, vars: impl IntoIterator<Item = (impl Into<String>, impl Into<OsString>)>) -> Self {
        self.env.extend(vars.into_iter().map(|(key, value)| (key.into(), value.into())));
        self
    }

    /// Sets the name fed to `${launcher_name}`.
    #[must_use]
    pub fn launcher_name(mut self, name: impl Into<String>) -> Self {
        self.launcher_name = name.into();
        self
    }

    /// Sets the version fed to `${launcher_version}`.
    #[must_use]
    pub fn launcher_version(mut self, version: impl Into<String>) -> Self {
        self.launcher_version = version.into();
        self
    }

    /// Validates the launch description eagerly and builds it.
    ///
    /// Validation: `java_path` must be an absolute path to an existing file, the
    /// working directory must exist and be a directory, the classpath must be
    /// non-empty when a main class is set, and the classpath must be joinable on
    /// the current platform. Errors carry the offending path where there is one.
    pub fn build(self) -> Result<Launcher, LaunchError> {
        if !self.java_path.is_absolute() || !self.java_path.is_file() {
            return Err(LaunchError::JavaNotFound { path: self.java_path });
        }
        if !self.working_directory.is_dir() {
            return Err(LaunchError::WorkingDirectoryMissing {
                path: self.working_directory,
            });
        }
        if !self.main_class.is_empty() && self.classpath.is_empty() {
            return Err(LaunchError::EmptyClasspath { main_class: self.main_class });
        }
        // Fail on separator-bearing entries now instead of at spawn time.
        env::join_paths(&self.classpath).map_err(|source| LaunchError::InvalidClasspath { source })?;

        Ok(Launcher {
            java_path: self.java_path,
            working_directory: self.working_directory,
            library_directory: self.library_directory,
            natives_directory: self.natives_directory,
            assets_directory: self.assets_directory,
            jvm_args: self.jvm_args,
            main_class: self.main_class,
            classpath: self.classpath,
            game_args: self.game_args,
            env: self.env,
            launcher_name: self.launcher_name,
            launcher_version: self.launcher_version,
        })
    }
}
