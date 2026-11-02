set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]
set shell := ["bash", "-c"]

@default:
    just --list

build:
    cargo build --release

# Builds Lodestone.app and a DMG for this Mac's architecture, into dist/. Run on macOS.
dmg:
    cargo build --release -p lodestone_launcher
    version="$(cargo pkgid -p lodestone_launcher | sed 's/.*[#@]//')" && cargo xtask bundle-macos --binary target/release/lodestone_launcher --version "$version" --out dist --dmg "dist/Lodestone-v${version}-macos.dmg"

# This will app update the version and publish a tag to github to initiate CICD
publish version="":
    uv -g commit-push-tag {{version}}

# Runs the launcher with the performance overlay (F3), the per-second report and the Chrome trace.
perf:
    cargo run -p lodestone_launcher --features perf

# Runs the performance instrumentation on a release-speed build with symbols, for sampling profilers.
profile:
    cargo run --profile profiling -p lodestone_launcher --features perf
