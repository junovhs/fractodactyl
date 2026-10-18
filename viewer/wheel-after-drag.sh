#!/usr/bin/env bash
# VIEW-09 check: in fd-viewer the mouse wheel still zooms after a drag-pan.
# Launches fd-viewer on X11, wheel-zooms over the fractal pane, drag-pans, wheel-zooms
# again, and compares the status bar's zoom readout after each step. Exits 0 only if
# the readout changes after the post-drag wheel. Needs xdotool, wmctrl and ImageMagick.
#   viewer/wheel-after-drag.sh [path/to/fd-viewer]
set -u
SELF="$(readlink -f "${BASH_SOURCE[0]}")"
ROOT="$(dirname "$(dirname "$SELF")")"
BIN="${1:-${CARGO_TARGET_DIR:-$ROOT/target}/release/fd-viewer}"
TMP="$(mktemp -d)"
PID=
trap 'st=$?; [ -n "$PID" ] && kill "$PID" 2>/dev/null; rm -rf "$TMP" 2>/dev/null; exit $st' EXIT

for t in xdotool wmctrl import compare; do
  command -v "$t" >/dev/null || { echo "missing $t" >&2; exit 2; }
done
[ -x "$BIN" ] || { echo "no fd-viewer at $BIN" >&2; exit 2; }

"$BIN" >"$TMP/viewer.log" 2>&1 &
PID=$!
W=
for _ in $(seq 1 60); do
  W="$(wmctrl -lp | awk -v p="$PID" '$3 == p && / Fractodactyl$/ {print $1}' | head -1)"
  [ -n "$W" ] && break
  sleep 0.5
done
[ -n "$W" ] || { echo "fd-viewer window did not appear" >&2; cat "$TMP/viewer.log" >&2; exit 2; }
wmctrl -ia "$W"
sleep 5   # first render
eval "$(xdotool getwindowgeometry --shell "$W")"
# The zoom readout sits in the status bar's right half (1280x820 window).
zoom() { import -window "$W" -crop "260x30+$((WIDTH - 400))+$((HEIGHT - 42))" "$TMP/$1.png"; }
same() { [ "$(compare -metric AE "$TMP/$1.png" "$TMP/$2.png" null: 2>&1 | cut -d' ' -f1)" = 0 ]; }
CX=$((X + WIDTH * 6 / 10)); CY=$((Y + HEIGHT * 6 / 10))

xdotool mousemove "$CX" "$CY"; sleep 0.3; zoom z0
xdotool click --repeat 3 --delay 80 4; sleep 1.5; zoom z1
if same z0 z1; then echo "FAIL: the wheel did not zoom even before a drag" >&2; exit 1; fi
xdotool mousedown 1; sleep 0.1
for _ in $(seq 1 10); do xdotool mousemove_relative 10 6; sleep 0.03; done
xdotool mouseup 1; sleep 1.5; zoom z2
xdotool click --repeat 3 --delay 80 4; sleep 1.5; zoom z3
if same z2 z3; then echo "FAIL: the wheel stopped zooming after a drag" >&2; exit 1; fi
echo "ok: the wheel zooms before and after a drag"
