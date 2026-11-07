#!/bin/sh
# Installs the latest Lodestone Minecraft Launcher release for Linux (x86_64) into ~/.local/bin, and adds a menu entry
# (a .desktop file and icon) so KDE Plasma, and any other freedesktop desktop, lists it in the application launcher.
#
#   curl -fsSL https://lodestonelauncher.com/install | sh
#
# The website serves this file at /install (see website/vite.config.ts), and the release ships it as installer.sh.
#
# Set BIN_DIR to install somewhere else. Set NO_DESKTOP_ENTRY=1 to skip the menu entry.

set -eu

REPO="Drew-Chase/lodestone-minecraft-launcher"
BINARY="lodestone_launcher"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
# The window's application id (see `application_id` in the launcher's boot.rs). Plasma matches a window to its menu
# entry through the entry's file name and StartupWMClass, so all three use this one name.
APP_ID="lodestone_launcher"

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

# The release's SHA256SUMS.txt lists the archive's hash. The archive is checked against it before it is unpacked.
sums_url=$(printf '%s\n' "$release_json" \
  | grep -o '"browser_download_url": *"[^"]*SHA256SUMS\.txt"' \
  | head -n 1 \
  | sed 's/.*"\(https[^"]*\)"$/\1/')
[ -n "$sums_url" ] || die "the latest release has no SHA256SUMS.txt, so the download cannot be checked."

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

archive_name=$(basename "$url")
printf 'Downloading %s\n' "$archive_name"
fetch "$url" "$tmp/archive.tar.gz" || die "the download failed."
fetch "$sums_url" "$tmp/SHA256SUMS.txt" || die "the checksums could not be downloaded."
expected=$(awk -v name="$archive_name" '{ file = $2; sub(/^\*/, "", file); if (file == name) { print $1; exit } }' "$tmp/SHA256SUMS.txt")
[ -n "$expected" ] || die "the checksums do not list $archive_name."
if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$tmp/archive.tar.gz" | awk '{ print $1 }')
else
  actual=$(shasum -a 256 "$tmp/archive.tar.gz" | awk '{ print $1 }')
fi
[ "$actual" = "$expected" ] || die "the download does not match its checksum."
tar -xzf "$tmp/archive.tar.gz" -C "$tmp"
[ -f "$tmp/$BINARY" ] || die "the archive does not contain $BINARY."

mkdir -p "$BIN_DIR"
install -m 0755 "$tmp/$BINARY" "$BIN_DIR/$BINARY"
printf 'Installed %s to %s\n' "$BINARY" "$BIN_DIR"

# The menu entry. The icon is fetched from the same tag as the release, and an icon that cannot be fetched only costs
# the entry its picture, so it never fails the install.
if [ -z "${NO_DESKTOP_ENTRY:-}" ]; then
  apps_dir="$DATA_DIR/applications"
  icon_dir="$DATA_DIR/icons/hicolor/512x512/apps"
  tag=$(printf '%s\n' "$release_json" \
    | grep -o '"tag_name": *"[^"]*"' \
    | head -n 1 \
    | sed 's/.*"\([^"]*\)"$/\1/')

  icon="$APP_ID"
  mkdir -p "$icon_dir"
  if fetch "https://raw.githubusercontent.com/$REPO/${tag:-main}/res/icon/icon.png" "$tmp/icon.png" 2>/dev/null \
    && [ -s "$tmp/icon.png" ]; then
    install -m 0644 "$tmp/icon.png" "$icon_dir/$APP_ID.png"
  else
    icon="applications-games"
    printf 'Could not download the icon, so the menu entry uses a stock one.\n'
  fi

  mkdir -p "$apps_dir"
  cat > "$apps_dir/$APP_ID.desktop" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=Lodestone
GenericName=Minecraft Launcher
Comment=Install, manage and play Minecraft instances
Exec="$BIN_DIR/$BINARY"
Icon=$icon
Terminal=false
Categories=Game;
Keywords=Minecraft;Launcher;Modpack;Mods;
StartupNotify=true
StartupWMClass=$APP_ID
EOF
  chmod 0644 "$apps_dir/$APP_ID.desktop"

  # Refresh the desktop's caches when their tools exist. Plasma also watches these folders, so it is fine if none do.
  if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$apps_dir" >/dev/null 2>&1 || true
  fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q -t -f "$DATA_DIR/icons/hicolor" >/dev/null 2>&1 || true
  fi
  if command -v kbuildsycoca6 >/dev/null 2>&1; then
    kbuildsycoca6 >/dev/null 2>&1 || true
  elif command -v kbuildsycoca5 >/dev/null 2>&1; then
    kbuildsycoca5 >/dev/null 2>&1 || true
  fi
  printf 'Added the menu entry %s\n' "$apps_dir/$APP_ID.desktop"
fi

case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) printf 'Add %s to your PATH to start it from a terminal.\n' "$BIN_DIR" ;;
esac
