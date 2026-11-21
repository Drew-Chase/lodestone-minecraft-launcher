#!/usr/bin/env bash
# Records the demo video's footage from a scratch launcher profile, then cuts it into frame folders for stage.html.
#
#   tools/demo-video/record.sh setup        build-independent: launch, sign in offline, skip setup, seed demo data
#   tools/demo-video/record.sh <clip>...    record clips: library platforms search resource newinst instance worlds servers skins stills
#   tools/demo-video/record.sh all          every clip, then extract
#   tools/demo-video/record.sh extract      cut the recordings into target/demo-video/stage/frames
#   tools/demo-video/record.sh stop         stop the scratch launcher
#
# Needs X11 through XWayland (DISPLAY=:0), xdotool, ffmpeg, ImageMagick and node. The launcher runs at 1600x900 and every
# coordinate below is a pixel in that window, measured against the v0.7.0 UI. When the UI moves, check a screenshot
# (`shot name`) and update the coordinates. Never kill the launcher by name: the user's own run uses the same binary.
set -uo pipefail

TOOLS=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO=$(cd "$TOOLS/../.." && pwd)
WORK=${DEMO_WORK:-$REPO/target/demo-video}
S=$WORK/profile
CAP=$WORK/capture
BIN=${LAUNCHER_BIN:-$REPO/target/debug/lodestone_launcher}
export DISPLAY=${DISPLAY_X11:-:0}
mkdir -p "$S/config" "$S/data" "$S/cache" "$CAP"

# ---------- helpers ----------
w() { timeout "$1" tail -f /dev/null || true; }
win() { W=$(xdotool search --pid "$(cat "$S/pid")" 2>/dev/null | tail -1); }
launch() {
  setsid -f sh -c 'echo $$ > "$0/pid"; exec env -u WAYLAND_DISPLAY -u XDG_SESSION_TYPE -u XDG_BACKEND -u APP_DIRECTORY WINIT_UNIX_BACKEND=x11 XDG_CONFIG_HOME="$0/config" XDG_DATA_HOME="$0/data" XDG_CACHE_HOME="$0/cache" "$1" </dev/null >"$0/out.log" 2>&1' "$S" "$BIN"
  w 6; W=$(xdotool search --sync --pid "$(cat "$S/pid")" | tail -1)
  xdotool windowsize "$W" 1600 900; w 1; xdotool windowactivate --sync "$W"; w 3
}
stop() { [ -f "$S/pid" ] && kill "$(cat "$S/pid")" 2>/dev/null; w 2; }
shot() { import -window "$W" "$CAP/$1.png"; }
mv_() { xdotool mousemove --window "$W" "$1" "$2"; }
clk() { mv_ "$1" "$2"; w 0.4; xdotool click 1; }
# Glide the pointer to x y over n steps, so a recording shows it travel instead of jump.
glide() {
  local X Y SCREEN WINDOW x0 y0 n=${3:-20}; eval "$(xdotool getmouselocation --shell)"; x0=$X; y0=$Y
  eval "$(xdotool getwindowgeometry --shell "$W")"; local tx=$((X + $1)) ty=$((Y + $2))
  for i in $(seq 1 "$n"); do xdotool mousemove $((x0 + (tx - x0) * i / n)) $((y0 + (ty - y0) * i / n)); w 0.015; done
}
gclk() { glide "$1" "$2" "${3:-20}"; w 0.25; xdotool click 1; }
# xdotool type drops characters in iced fields, so keys go one at a time.
keys() { xdotool windowfocus --sync "$W"; local s="$1" i c; for ((i = 0; i < ${#s}; i++)); do c="${s:$i:1}"
  case "$c" in " ") xdotool key space ;; ".") xdotool key period ;; [A-Z]) xdotool key "shift+$c" ;; *) xdotool key "$c" ;; esac; w 0.07; done; }
# Scroll smoothly by dragging the scrollbar thumb one pixel per tick. The mouse wheel moves iced lists ~60 px a click,
# which looks like teleporting on video.
sdrag() { local x=$1 y=$2 dy=$3 d=${4:-0.02} st=1; mv_ "$x" "$y"; w 0.3; xdotool mousedown 1; w 0.1; [ "$dy" -lt 0 ] && st=-1
  for i in $(seq 1 "${dy#-}"); do xdotool mousemove_relative -- 0 $st; w "$d"; done; w 0.1; xdotool mouseup 1; }
rec_start() { ffmpeg -hide_banner -loglevel error -y -f x11grab -framerate 30 -window_id "$W" -i "$DISPLAY" -c:v libx264 -preset ultrafast -crf 14 -pix_fmt yuv420p "$CAP/$1.mp4" </dev/null & REC=$!; w 0.5; }
rec_stop() { kill -INT "$REC"; wait "$REC" 2>/dev/null; w 0.5; }
ensure() { win; [ -n "${W:-}" ] || launch; xdotool windowactivate --sync "$W"; }

# ---------- setup ----------
setup() {
  stop; launch
  clk 617 385; w 1                       # Offline Mode
  clk 790 283; w 0.4; keys "AlexPlays"   # username
  clk 617 338; w 2                       # Play offline
  clk 500 779; w 2                       # Skip setup
  stop
  node "$TOOLS/seed.mjs" "$S" "$REPO"
  launch
}

# ---------- clips (each starts from the page it needs) ----------
library() { ensure; clk 39 100; w 2; sdrag 1591 450 -300 0.002; mv_ 800 250; w 1.5
  rec_start r_library; w 0.8; glide 185 318 30; w 0.8; glide 340 720 30; w 0.7; glide 840 720 30; w 0.7; glide 1330 720 30; w 0.6
  sdrag 1591 330 240 0.012; w 0.6; glide 840 650 30; w 0.6; glide 340 600 30; w 0.6; sdrag 1591 570 -240 0.008; w 0.4; glide 700 300 30; w 0.8; rec_stop; }
platforms() { ensure; clk 39 155; w 4; clk 228 112; w 4; mv_ 900 330; w 1
  rec_start r_discover_platforms; w 0.8; gclk 340 112 25; w 3; gclk 482 112 20; w 2.8; gclk 610 112 20; w 2.6; gclk 707 112 20; w 2.6; gclk 228 112 20; w 3; rec_stop; }
search() { ensure; clk 39 155; w 3; clk 228 112; w 3; clk 355 171; w 3; mv_ 900 330; w 1
  rec_start r_discover_search; w 0.8; gclk 1300 50 25; w 0.5; keys "sodium"; w 3; glide 590 700 25; w 1; glide 990 720 20; w 1; glide 1380 720 20; w 1.2; rec_stop
  xdotool key ctrl+a BackSpace; w 1; clk 186 171; w 2; }
resource() { ensure; clk 39 155; w 3; clk 228 112; w 3; clk 186 171; w 4; mv_ 900 470; w 1
  rec_start r_resource; w 0.5; gclk 478 419 25; w 3.5; glide 1591 320 30; w 0.3; sdrag 1591 320 160 0.02; w 0.8; sdrag 1591 480 -160 0.006; w 0.3; gclk 248 255 30; w 3.5; rec_stop; }
newinst() { ensure; clk 39 100; w 2; mv_ 700 400; w 1
  rec_start r_newinst; w 0.6; gclk 280 494 25; w 1.5; gclk 840 376 20; w 0.4; keys "Aether Quest"; w 0.8; gclk 760 630 20; w 1.2; gclk 945 630 15; w 1.2
  gclk 605 483 20; w 1.5; keys "1.21"; w 1.5; xdotool key Down; w 0.3; xdotool key Down; w 0.6; xdotool key Return; w 1.5; glide 1100 820 25; w 1.5; rec_stop
  xdotool key Escape; w 1; }   # never press Create Instance: it starts a real install
instance() { ensure; clk 39 100; w 2; mv_ 800 400; w 0.3; xdotool click 1; w 0.5; clk 300 690; w 3; mv_ 900 200; w 1
  rec_start r_instance; w 0.8; glide 1591 520 30; w 0.3; sdrag 1591 520 130 0.022; w 0.6; sdrag 1591 650 -130 0.008; w 0.3
  gclk 229 326 30; w 2.8; gclk 307 326 25; w 3.0; gclk 593 326 25; w 2.5; glide 900 600 30; w 1.5; rec_stop; }
worlds() { ensure; clk 39 209; w 3; mv_ 900 450; w 1
  rec_start r_worlds; w 0.6; glide 340 260 30; w 0.8; glide 835 260 25; w 0.6; glide 1591 400 30; w 0.2; sdrag 1591 400 150 0.025; w 0.6; sdrag 1591 550 -150 0.01; w 0.3
  gclk 340 94 30; w 1.5; gclk 538 94 25; w 1.5; gclk 156 94 25; w 1; gclk 1551 95 30; w 2; gclk 1512 95 15; w 1.5; rec_stop; }
servers() { ensure; clk 39 317; w 6; mv_ 900 450; w 1
  rec_start r_servers; w 0.6; glide 1350 215 25; w 1; glide 1350 370 15; w 1; glide 1520 668 20; w 1; gclk 1398 52 20; w 3.5; glide 800 500 20; w 1; rec_stop; }
skins() { ensure; clk 39 263; w 3; mv_ 400 400; w 1
  rec_start r_skins; w 0.6; xdotool mousedown 1; for i in $(seq 1 45); do xdotool mousemove_relative -- 9 0; w 0.03; done
  for i in $(seq 1 25); do xdotool mousemove_relative -- -6 1; w 0.03; done; xdotool mouseup 1; w 0.6
  gclk 680 402 20; w 2; gclk 680 450 10; w 2; gclk 935 530 25; w 2; rec_stop; }
stills() { ensure; clk 39 100; w 2
  clk 650 494; w 2; mv_ 1590 890; w 1; shot m_import; xdotool key Escape; w 1
  clk 1020 494; w 2; mv_ 1590 890; w 1; shot m_join; xdotool key Escape; w 1
  clk 1530 576; w 2; mv_ 1590 890; w 1; shot m_table; clk 1350 576; w 1; mv_ 1590 890; w 1; shot library
  clk 39 209; w 3; mv_ 1590 890; w 1; shot p_worlds
  clk 39 807; w 3; clk 193 184; w 2; mv_ 1590 890; w 1; shot m_java; clk 181 232; w 2; mv_ 1590 890; w 1; shot m_appearance
  clk 39 155; w 3; clk 340 112; w 6; mv_ 1590 890; w 1; shot m_discover; clk 228 112; w 2; clk 39 100; w 1; }   # CurseForge results, for the thumbnail

# ---------- extract ----------
# name, recording, in, out (seconds in the recording). stage.html's SEG table sets each clip's rate and trim on top of this.
SEGMENTS="library r_library 0.6 16.8
platforms r_discover_platforms 0.6 17.0
search r_discover_search 2.6 10.3
resource r_resource 0.4 16.8
newinst r_newinst 0.6 18.0
instance r_instance 0.6 21.0
worlds r_worlds 10.5 23.3
servers r_servers 0.6 10.2
skins r_skins 0.6 11.6"
extract() {
  local F=$WORK/stage/frames manifest="window.FRAMES = {"; mkdir -p "$F"
  while read -r name src a b; do
    rm -rf "${F:?}/$name"; mkdir -p "$F/$name"
    ffmpeg -loglevel error -y -ss "$a" -to "$b" -i "$CAP/$src.mp4" -vf fps=30 -q:v 3 "$F/$name/%05d.jpg"
    manifest+=" $name: $(ls "$F/$name" | wc -l),"; echo "$name $(ls "$F/$name" | wc -l) frames"
  done <<< "$SEGMENTS"
  echo "${manifest%,} };" > "$F/manifest.js"
  for s in m_import m_join m_table m_java m_appearance m_discover p_worlds library; do magick "$CAP/$s.png" -quality 92 "$F/$s.jpg"; done
}

case "${1:-}" in
  setup) setup ;;
  stop) stop ;;
  extract) extract ;;
  all) for c in library platforms search resource newinst instance worlds servers skins stills; do echo "recording $c"; "$c"; done; extract ;;
  library|platforms|search|resource|newinst|instance|worlds|servers|skins|stills) for c in "$@"; do echo "recording $c"; "$c"; done ;;
  *) sed -n '2,13p' "$0"; exit 1 ;;
esac
