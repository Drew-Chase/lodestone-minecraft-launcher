set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]
set shell := ["bash", "-c"]

[windows]
build docker="false":
    @just {{ if docker == "true" { "_build_docker" } else { "_build" } }}

[linux]
build: _build

_build:
    cargo build --release

[windows]
_build_docker:
    New-Item -ItemType Directory -Force -Path target/docker, target/release | Out-Null
    docker build -t lodestone-builder -f Dockerfile .
    docker run --rm -v "${PWD}:/workspace" -v "${PWD}/target/docker:/target" lodestone-builder
    Copy-Item -Force target/docker/release/lodestone-launcher target/release/lodestone-launcher
    Copy-Item -Force target/docker/release/lodestone_website target/release/lodestone_website

publish version="":
    uv -g commit-push-tag {{ version }}

run platform="app":
    @just {{ if platform == "website" { "_run_website" } else { "_run_app" } }}

# Build only the website (frontend + backend binary)
build-website:
    @just _build_website_frontend
    cargo build --release --package lodestone_website

[windows]
_build_website_frontend:
    cd crates/lodestone-website; pnpm install && pnpm run build-frontend

[linux]
[macos]
_build_website_frontend:
    cd crates/lodestone-website && pnpm install && pnpm run build-frontend

[windows]
_run_app:
    cd crates/lodestone-gui; pnpm install && pnpm run tauri-dev

[windows]
_run_website:
    cd crates/lodestone-website; pnpm install
    cargo run --package lodestone_website

[linux]
[macos]
_run_app:
    cd crates/lodestone-gui && pnpm install && pnpm run tauri-dev

[linux]
[macos]
_run_website:
    cd crates/lodestone-website && pnpm install
    cargo run --package lodestone_website
