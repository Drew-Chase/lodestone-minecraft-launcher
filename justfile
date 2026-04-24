set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile","-Command"]
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
    Copy-Item -Force target/docker/release/lodestone target/release/lodestone
