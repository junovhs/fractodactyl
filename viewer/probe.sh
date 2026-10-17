#!/usr/bin/env bash
# VIEW-08: does the explorer reuse its tiles? Starts fd explore and headless Chrome on
# /?probe=1, which settles at Seahorse valley, pans half a screen, then zooms in 2x, and
# reports what each step rendered in screens of full-quality samples. Build fd first
# (cargo build --release -p fd-cli); FD=path and CHROME=browser override the defaults.
set -euo pipefail
cd "$(dirname "$0")/.."
FD=${FD:-target/release/fd}
CHROME=${CHROME:-google-chrome}
PORT=${PORT:-8791}
tmp=$(mktemp -d)
# Chrome forks helpers that outlive the job: stop them all by profile, keep the exit status.
trap 'rc=$?; kill $(jobs -p) 2>/dev/null; pkill -f "user-data-dir=$tmp/chrome" 2>/dev/null; wait 2>/dev/null; sleep 0.3; rm -rf "$tmp" 2>/dev/null; exit $rc' EXIT
"$FD" explore --port "$PORT" --capture "$tmp/probe.json" >"$tmp/fd.log" 2>&1 &
for _ in $(seq 100); do curl -sf "http://127.0.0.1:$PORT/looks" >/dev/null && break; sleep 0.1; done
"$CHROME" --headless=new --no-first-run --user-data-dir="$tmp/chrome" --window-size=1280,720 \
  --use-angle=swiftshader --enable-unsafe-swiftshader "http://127.0.0.1:$PORT/?probe=1" >"$tmp/chrome.log" 2>&1 &
for _ in $(seq 1200); do [ -s "$tmp/probe.json" ] && break; sleep 0.1; done
[ -s "$tmp/probe.json" ] || { echo "probe: no result after 120 s (old page without ?probe, or fd explore failed)"; exit 1; }
python3 - "$tmp/probe.json" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
print(f"view {r['W']}x{r['H']}: one full-quality screen = {r['screenTiles']} tiles")
print(f"half-screen pan rendered {r['pan']:.2f} screens (pass <= 0.75)")
print(f"2x zoom-in rendered {r['zoomIn']:.2f} screens; base-quality tiles missing right after it: "
      f"{r['baseMissingAfterZoomIn']} (pass = 0)")
ok = r['pan'] <= 0.75 and r['baseMissingAfterZoomIn'] == 0
print("PASS" if ok else "FAIL")
sys.exit(0 if ok else 1)
PY
