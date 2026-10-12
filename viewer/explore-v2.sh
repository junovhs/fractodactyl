#!/usr/bin/env bash
# Linux V2 shares the genuine fd renderer with the classic explorer.
set -e
SELF="$(readlink -f "$0")"
VIEWER="$(dirname "$SELF")"

case "$1" in
  --install)
    APPS="$HOME/.local/share/applications"
    mkdir -p "$APPS"
    ENTRY="$APPS/fractodactyl-explorer-v2.desktop"
    printf '[Desktop Entry]\nType=Application\nName=Fractodactyl Explorer V2\nComment=Explore Mandelbrot with the stone interface\nExec="%s"\nIcon=applications-graphics\nTerminal=false\nCategories=Graphics;\n' "$SELF" > "$ENTRY"
    echo "installed $ENTRY"
    DESKTOP="$(xdg-user-dir DESKTOP 2>/dev/null || echo "$HOME/Desktop")"
    if [ -d "$DESKTOP" ]; then
      cp "$ENTRY" "$DESKTOP/fractodactyl-explorer-v2.desktop"
      chmod +x "$DESKTOP/fractodactyl-explorer-v2.desktop"
      command -v gio >/dev/null && gio set "$DESKTOP/fractodactyl-explorer-v2.desktop" metadata::trusted true 2>/dev/null || true
      echo "installed $DESKTOP/fractodactyl-explorer-v2.desktop"
    fi
    ;;
  --stop)
    exec "$VIEWER/explore.sh" --stop
    ;;
  *)
    exec "$VIEWER/explore.sh" --v2
    ;;
esac
