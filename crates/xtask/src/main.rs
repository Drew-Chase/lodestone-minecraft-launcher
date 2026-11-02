//! `cargo xtask <task>`: the build tasks behind the release artifacts. The alias lives in `.cargo/config.toml`.

use std::process::ExitCode;

use xtask::macos;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.split_first() {
        Some((task, rest)) if task == "bundle-macos" => macos::options(rest).and_then(|options| macos::bundle(&options)),
        _ => Err(macos::USAGE.to_string()),
    };
    match result {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}
