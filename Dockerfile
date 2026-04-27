FROM rust:1-slim-bookworm
LABEL authors="drew.chase"

# Build-time system dependencies. Split into two groups:
#   1. General Rust/crate build deps (pkg-config, libssl-dev, build-essential, git).
#   2. Tauri 2 Linux bundling deps (webkit2gtk 4.1, gtk/appindicator/rsvg, xdo,
#      patchelf for AppImage relinking, and curl/wget/file used by the bundler).
#   3. musl-tools for building a fully static website binary.
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
        musl-tools \
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

# Install the musl target for a fully static website binary that runs on any Linux
# (CentOS, Alpine, older Ubuntu, etc.) without glibc version requirements.
RUN rustup target add x86_64-unknown-linux-musl

# Write cargo artifacts to /target so the host bind-mount preserves debug/release
# builds (and the incremental cache) across container runs. Tauri's bundler
# writes its output to ${CARGO_TARGET_DIR}/release/bundle/ as well.
ENV CARGO_TARGET_DIR=/target

# CI=true suppresses interactive prompts (pnpm module purge confirmation, etc.).
ENV CI=true

WORKDIR /workspace

# BUILD_TARGET controls what to build: "all" (default), "app", or "website".
# Passed via: docker run -e BUILD_TARGET=website ...
ENV BUILD_TARGET=all

# The entrypoint script reads BUILD_TARGET and runs the appropriate build(s).
#
# pnpm v10+ blocks dependency postinstall/build scripts by default for security.
# The .npmrc files in each crate allowlist the packages that need their scripts.
#
# App build:     Tauri desktop bundle (glibc-linked, normal for desktop Linux).
# Website build: Static musl binary with no glibc/libssl dependency.
CMD ["bash", "-c", "\
  set -e; \
  if [ \"$BUILD_TARGET\" = 'all' ] || [ \"$BUILD_TARGET\" = 'app' ]; then \
    echo '=== Building Tauri app ===' \
    && cd crates/lodestone-gui && pnpm install && pnpm run tauri-build \
    && cd ../..; \
  fi; \
  if [ \"$BUILD_TARGET\" = 'all' ] || [ \"$BUILD_TARGET\" = 'website' ]; then \
    echo '=== Building website ===' \
    && cd crates/lodestone-website && pnpm install && pnpm run build-frontend \
    && cd ../.. \
    && cargo build --release --package lodestone_website --target x86_64-unknown-linux-musl; \
  fi; \
  echo '=== Build complete ===' \
"]
