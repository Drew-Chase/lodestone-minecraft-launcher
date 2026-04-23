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
WORKDIR /workspace

# Build the Tauri desktop bundle for Linux. pnpm install hydrates node_modules
# (including @tauri-apps/cli) and then the `tauri-build` script from
# crates/lodestone-gui/package.json runs `tauri build`, which in turn triggers
# the Vite frontend build via beforeBuildCommand.
CMD ["bash", "-lc", "cd crates/lodestone-gui && pnpm install && pnpm run tauri-build"]
