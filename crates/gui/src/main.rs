#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod app;
mod logging;

use std::process::ExitCode;

fn main() -> ExitCode {
    if let Err(e) = logging::setup_logging() {
        eprintln!(": {e}");
    }
    if let Err(e) = color_eyre::install() {
        eprintln!("failed to install color-eyre: {e}");
    }

    match lodestone_launcher::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", e);
            ExitCode::FAILURE
        }
    }
}
