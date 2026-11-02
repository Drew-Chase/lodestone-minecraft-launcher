#!/bin/sh
# Installs the latest Lodestone Minecraft Launcher release for Linux (x86_64) into ~/.local/bin.
#
#   curl -fsSL https://github.com/Drew-Chase/lodestone-minecraft-launcher/releases/latest/download/installer.sh | sh
#
# Set BIN_DIR to install somewhere else.

set -eu

REPO="Drew-Chase/lodestone-minecraft-launcher"
BINARY="lodestone_launcher"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"

die() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

[ "$(uname -s)" = "Linux" ] || die "this installer is for Linux. Get the build for your system from the release page."
[ "$(uname -m)" = "x86_64" ] || die "the Linux build is x86_64 only, and this machine is $(uname -m)."

if command -v curl >/dev/null 2>&1; then
  fetch() { curl -fsSL -o "$2" "$1"; }
  fetch_text() { curl -fsSL "$1"; }
elif command -v wget >/dev/null 2>&1; then
  fetch() { wget -q -O "$2" "$1"; }
  fetch_text() { wget -q -O - "$1"; }
else
  die "curl or wget is needed to download the release."
fi

# The release's Linux archive is the asset whose name ends in linux-x86_64.tar.gz. The API answer is read with grep
# and sed, so the installer needs no JSON tool.
release_json=$(fetch_text "https://api.github.com/repos/$REPO/releases/latest") \
  || die "could not reach GitHub for the latest release."
url=$(printf '%s\n' "$release_json" \
  | grep -o '"browser_download_url": *"[^"]*linux-x86_64\.tar\.gz"' \
  | head -n 1 \
  | sed 's/.*"\(https[^"]*\)"$/\1/')
[ -n "$url" ] || die "the latest release has no Linux x86_64 archive."

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

printf 'Downloading %s\n' "$(basename "$url")"
fetch "$url" "$tmp/archive.tar.gz" || die "the download failed."
tar -xzf "$tmp/archive.tar.gz" -C "$tmp"
[ -f "$tmp/$BINARY" ] || die "the archive does not contain $BINARY."

mkdir -p "$BIN_DIR"
install -m 0755 "$tmp/$BINARY" "$BIN_DIR/$BINARY"
printf 'Installed %s to %s\n' "$BINARY" "$BIN_DIR"

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) printf 'Add %s to your PATH to start it from a terminal.\n' "$BIN_DIR" ;;
esac
