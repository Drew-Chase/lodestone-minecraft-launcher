FROM rust:1-slim-bookworm
LABEL authors="drew.chase"

# Build-time system dependencies. Split into two groups:
#   1. General Rust/crate build deps (pkg-config, libssl-dev, build-essential, git).
#   2. Tauri 2 Linux bundling deps (webkit2gtk 4.1, gtk/appindicator/rsvg, xdo,
#      patchelf for AppImage relinking, and curl/wget/file used by the bundler).
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        build-essential \
        ca-certificates \
        git \
        curl \
        wget \
        file \
        libwebkit2gtk-4.1-dev \
        libxdo-dev \
        libayatana-appindicator3-dev \
        librsvg2-dev \
        patchelf \
 && rm -rf /var/lib/apt/lists/*

# Node.js 20.x is required to run the Vite frontend build that Tauri invokes via
# `beforeBuildCommand: pnpm build` in tauri.conf.json.
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
 && apt-get install -y --no-install-recommends nodejs \
 && rm -rf /var/lib/apt/lists/*

# Enable pnpm through corepack. package.json pins `engines.pnpm >=8.13.1`; latest
# satisfies that and keeps us on a supported release.
RUN corepack enable \
 && corepack prepare pnpm@latest --activate

# Write cargo artifacts to /target so the host bind-mount preserves debug/release
# builds (and the incremental cache) across container runs. Tauri's bundler
# writes its output to ${CARGO_TARGET_DIR}/release/bundle/ as well.
ENV CARGO_TARGET_DIR=/target

# CI=true suppresses interactive prompts (pnpm module purge confirmation, etc.).
ENV CI=true

WORKDIR /workspace

# Build both the Tauri desktop app and the website backend.
#
# pnpm v10+ blocks dependency postinstall/build scripts by default for security.
# The .npmrc files in each crate allowlist the packages that need their scripts:
#   - @tauri-apps/cli: downloads the platform-specific Tauri CLI native binary
#   - esbuild: installs the platform-specific WASM/native bundler binary
#   - @parcel/watcher: native file-watcher addon
#   - @heroui/shared-utils: build step for the UI library
#
# Flow:
#   1. Tauri app: pnpm install + tauri build (frontend via beforeBuildCommand,
#      then cargo builds the Tauri app bundle).
#   2. Website: pnpm install + vite build (frontend into target/wwwroot), then
#      cargo build compiles the Actix binary which embeds the frontend via
#      include_dir!().
CMD ["bash", "-c", "\
  cd crates/lodestone-gui && pnpm install && pnpm run tauri-build \
  && cd ../lodestone-website && pnpm install && pnpm run build-frontend \
  && cd ../.. && cargo build --release --package lodestone_website \
"]
