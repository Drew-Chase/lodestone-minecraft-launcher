set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]
set shell := ["bash", "-c"]

# Build using Docker. Pass target: "all" (default), "app", or "website".
#   just build              — native cargo build
#   just build docker       — Docker build (both app + website)
#   just build docker app   — Docker build (Tauri app only)
#   just build docker website — Docker build (website only)
[windows]
build docker="false" target="all":
    @just {{ if docker == "docker" { "_build_docker " + target } else { "_build" } }}

[linux]
build: _build

_build:
    cargo build --release

[windows]
_build_docker target="all":
    New-Item -ItemType Directory -Force -Path target/docker, target/release | Out-Null
    docker build -t lodestone-builder -f Dockerfile .
    docker run --rm -e BUILD_TARGET={{ target }} -v "${PWD}:/workspace" -v "${PWD}/target/docker:/target" lodestone-builder
    @just _copy_docker_artifacts {{ target }}

[windows]
_copy_docker_artifacts target="all":
    {{ if target == "all" { "Copy-Item -Force target/docker/release/lodestone-launcher target/release/lodestone-launcher" } else if target == "app" { "Copy-Item -Force target/docker/release/lodestone-launcher target/release/lodestone-launcher" } else { "" } }}
    {{ if target == "all" { "Copy-Item -Force target/docker/x86_64-unknown-linux-musl/release/lodestone_website target/release/lodestone_website" } else if target == "website" { "Copy-Item -Force target/docker/x86_64-unknown-linux-musl/release/lodestone_website target/release/lodestone_website" } else { "" } }}

publish version="":
    uv -g commit-push-tag {{ version }}

run platform="app":
    @just {{ if platform == "website" { "_run_website" } else { "_run_app" } }}

# Build only the website (frontend + backend binary) natively
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
    cd crates/lodestone-gui; pnpm install; pnpm run tauri-dev

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
