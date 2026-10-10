#!/bin/bash
# Web mode: send text to ChatGPT in Chrome (docs/spec/WEB-MODE.md). X11 only (xdotool, wmctrl, xclip).
#   send.sh new ID [--note FILE]   build the brief for ID and send it in a NEW chat
#   send.sh reply FILE             paste FILE into the CURRENT chat and send it
# Coordinates fit the owner's screen (Chrome window at the left, 1260 px wide): check a screenshot if the layout changes.
set -e
here=$(cd "$(dirname "$0")" && pwd); D=${WEB_DIR:-/tmp/web-mode}
W=${WEB_WIN:-$(wmctrl -l | grep -E "ChatGPT|Google Chrome" | grep -v github | head -1 | cut -d' ' -f1)}
[ -n "$W" ] || { echo "no Chrome window found; set WEB_WIN"; exit 1; }
paste_send() {  # $1 file, $2 x, $3 y of the composer
  xclip -selection clipboard < "$1"
  xdotool mousemove "$2" "$3" click 1; sleep 0.4
  xdotool key --clearmodifiers ctrl+a; xdotool key BackSpace; sleep 0.3   # drop any stale draft
  xdotool key --clearmodifiers ctrl+v; sleep 2; xdotool key Return; sleep 4
}
case "$1" in
  new)
    python3 "$here/brief.py" "$2" "${@:3}" --out "$D"
    wmctrl -i -a "$W"; sleep 0.5
    xdotool key --clearmodifiers ctrl+l; sleep 0.3; xdotool type --delay 15 "https://chatgpt.com/"; xdotool key Return; sleep 6
    paste_send "$D/brief-$2.txt" 600 982 ;;
  reply)
    wmctrl -i -a "$W"; sleep 0.5; paste_send "$2" 600 1990 ;;
  *) echo "usage: send.sh new ID [--note FILE] | reply FILE"; exit 2 ;;
esac
xdotool getwindowname "$W" | cut -c1-60
