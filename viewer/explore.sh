#!/usr/bin/env bash
# Mandelbrot explorer (EXPL-01), Linux twin of explore.cmd: builds fd if needed,
# starts `fd explore` in the background (unless it is already running) and opens
# it in Chrome. Options:
#   --install  add a "Fractodactyl Explorer" icon to the app menu and ~/Desktop
#   --stop     stop the background renderer
# Server log: ${XDG_RUNTIME_DIR:-/tmp}/fd-explore.log
set -u
PORT=8737
URL="http://127.0.0.1:$PORT/"
SELF="$(readlink -f "${BASH_SOURCE[0]}")"
ROOT="$(dirname "$(dirname "$SELF")")"
FD="${CARGO_TARGET_DIR:-$ROOT/target}/release/fd"
LOG="${XDG_RUNTIME_DIR:-/tmp}/fd-explore.log"

fail() {
  echo "$1" >&2
  command -v notify-send >/dev/null && notify-send "Fractodactyl Explorer" "$1"
  exit 1
}

up() { (exec 3<>"/dev/tcp/127.0.0.1/$PORT") 2>/dev/null; }

case "${1:-}" in
  --install)
    entry="[Desktop Entry]
Type=Application
Name=Fractodactyl Explorer
Comment=Explore the Mandelbrot set with fd explore
Exec=\"$SELF\"
Icon=applications-graphics
Terminal=false
Categories=Graphics;"
    apps="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
    mkdir -p "$apps"
    printf '%s\n' "$entry" > "$apps/fractodactyl-explorer.desktop"
    echo "installed $apps/fractodactyl-explorer.desktop"
    desk="$(xdg-user-dir DESKTOP 2>/dev/null || echo "$HOME/Desktop")"
    if [ -d "$desk" ]; then
      f="$desk/fractodactyl-explorer.desktop"
      printf '%s\n' "$entry" > "$f"
      chmod +x "$f"
      # GNOME only launches desktop icons marked trusted.
      command -v gio >/dev/null && gio set "$f" metadata::trusted true 2>/dev/null
      echo "installed $f"
    fi
    exit 0 ;;
  --stop)
    pkill -f "/fd explore --port $PORT\$" && echo "stopped" || echo "not running"
    exit 0 ;;
esac

cd "$ROOT" || fail "cannot cd to $ROOT"
if command -v cargo >/dev/null; then
  cargo build --release -p fd-cli >>"$LOG" 2>&1 || fail "cargo build failed; see $LOG"
fi
[ -x "$FD" ] || fail "fd was not built; install Rust (cargo) and try again."

if ! up; then
  setsid "$FD" explore --port "$PORT" >>"$LOG" 2>&1 < /dev/null &
  for _ in $(seq 50); do up && break; sleep 0.1; done
  up || fail "fd explore did not start; see $LOG"
fi

for b in google-chrome google-chrome-stable chromium chromium-browser; do
  if command -v "$b" >/dev/null; then
    setsid "$b" "$URL" >/dev/null 2>&1 < /dev/null &
    exit 0
  fi
done
setsid xdg-open "$URL" >/dev/null 2>&1 < /dev/null &
