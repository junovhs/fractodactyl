#!/bin/bash
# Web mode: send text to ChatGPT in Chrome (docs/spec/WEB-MODE.md). X11 only (xdotool, wmctrl, xclip).
#   send.sh new ID [--note FILE]   build the brief for ID and send it in a NEW chat
#   send.sh reply ID FILE          send FILE into ID's chat (reopened from its saved URL)
# Plain keystrokes, as a person would type. Each send opens its own tab, so the owner's tabs are
# never touched; ChatGPT focuses its message box on load. Nothing is read from the page except
# the chat's URL (address bar), saved for replies. Do not use Shift+Esc: Chrome takes it.
set -e
here=$(cd "$(dirname "$0")" && pwd); D=${WEB_DIR:-/tmp/web-mode}; mkdir -p "$D"
W=${WEB_WIN:-$(wmctrl -l | grep "Google Chrome" | grep -v -E "Task Manager|github" | head -1 | cut -d' ' -f1)}
[ -n "$W" ] || { echo "no Chrome window found; set WEB_WIN"; exit 1; }
open_tab() {  # $1 url
  wmctrl -i -a "$W"; sleep 0.5
  xdotool key --clearmodifiers ctrl+t; sleep 0.5
  xdotool type --delay 15 "$1"; xdotool key Return; sleep 8
}
paste_send() {  # $1 file
  xclip -selection clipboard < "$1"
  xdotool key --clearmodifiers ctrl+a; xdotool key BackSpace; sleep 0.3   # drop any stale draft
  xdotool key --clearmodifiers ctrl+v; sleep 2; xdotool key Return; sleep 2
}
case "$1" in
  new)
    python3 "$here/brief.py" "$2" "${@:3}" --out "$D"
    open_tab "https://chatgpt.com/"; paste_send "$D/brief-$2.txt"
    sleep 8; xdotool key --clearmodifiers ctrl+l; xdotool key --clearmodifiers ctrl+c; xdotool key Escape; sleep 0.3
    url=$(xclip -selection clipboard -o)
    case "$url" in https://chatgpt.com/c/*) echo "$url" > "$D/chat-$2.url"; echo "chat: $url" ;;
      *) echo "warning: no chat URL yet ($url); save it to $D/chat-$2.url before replying" ;; esac ;;
  reply)
    [ -s "$D/chat-$2.url" ] || { echo "no saved chat URL: $D/chat-$2.url"; exit 1; }
    open_tab "$(cat "$D/chat-$2.url")"; paste_send "$3" ;;
  *) echo "usage: send.sh new ID [--note FILE] | reply ID FILE"; exit 2 ;;
esac
