//! Manual smoke launch against a real instance directory (DESIGN.md §9).
//!
//! This is a human-driven test bench, not an automated test: point it at any
//! instance you have on disk and watch the event stream. The pump loop below is
//! the reference implementation gui's `Subscription` should mirror.
//!
//! Usage:
//! `cargo run -p lodestone_launch --example vanilla -- <java> <instance-dir> <main-class> [classpath-entry ...]`
//!
//! The crate does not substitute authentication placeholders: composing
//! `${auth_player_name}` and friends from session data is the caller's job.

use std::path::PathBuf;

use lodestone_launch::{LaunchError, LaunchEvent, Launcher};

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), LaunchError> {
    let mut args = std::env::args_os().skip(1);
    let (Some(java_path), Some(instance_dir), Some(main_class)) = (args.next(), args.next(), args.next()) else {
        eprintln!("usage: vanilla <java> <instance-dir> <main-class> [classpath-entry ...]");
        std::process::exit(2);
    };

    let java_path = PathBuf::from(java_path);
    let instance_dir = PathBuf::from(instance_dir);
    let main_class = main_class.to_string_lossy().into_owned();
    let classpath: Vec<PathBuf> = args.map(PathBuf::from).collect();

    let launcher = Launcher::builder(&java_path, &instance_dir)
        .library_directory(instance_dir.join("libraries"))
        .natives_directory(instance_dir.join("natives"))
        .assets_directory(instance_dir.join("assets"))
        .jvm_args(["-Xmx1G"])
        .main_class(main_class)
        .classpath(classpath)
        .launcher_name("lodestone_launch-vanilla-example")
        .launcher_version(env!("CARGO_PKG_VERSION"))
        .build()?;

    println!("launching {}", launcher.java_path().display());
    let mut game = launcher.launch()?;
    let events = game.take_events();

    while let Ok(event) = events.recv() {
        match event {
            LaunchEvent::Log { stream, line } => println!("[{stream:?}] {line}"),
            LaunchEvent::Exited { status } => println!("game exited: {status}"),
        }
    }

    // `Disconnected` means the game exited and both pipes ended; the status is
    // already collected at that point, so `wait` is instant.
    let status = game.wait()?;
    println!("exit status: {status}");
    Ok(())
}
