#!/usr/bin/env bash
# SITE-04 check: the home page's top nav stays on the page (anchors), except one Docs
# link and GitHub; no desktop/taskbar layout; and the page's colour pairs meet WCAG AA.
#   ./check-home.sh [URL]   (default https://fractodactyl.dev/)
set -u
URL="${1:-https://fractodactyl.dev/}"
HTML="$(curl -fsSL --max-time 20 "$URL")" || { echo "FAIL: could not fetch $URL"; exit 1; }
python3 - "$HTML" <<'EOF'
import re, sys
html = sys.argv[1]
fail = []
head = re.search(r"<header[^>]*>(.*?)</header>", html, re.S)
if not head:
    fail.append("no <header>")
else:
    hrefs = re.findall(r'href="([^"]+)"', head.group(1))
    others = [h for h in hrefs if not h.startswith("#") and h != "/"]
    docs = [h for h in others if h.startswith("/docs")]
    rest = [h for h in others if not h.startswith("/docs") and "github.com" not in h]
    if docs != ["/docs/"]: fail.append(f"header docs links {docs} (want exactly one /docs/)")
    if rest: fail.append(f"header links leave the page: {rest}")
for bad in ('class="taskbar"', 'class="icons"', "confetti"):
    if bad in html: fail.append(f"page still has {bad}")
# WCAG contrast of the page's declared pairs: /* aa: --fg on --bg */ comments in the CSS.
tokens = dict(re.findall(r"(--c-[\w-]+):\s*(#[0-9a-fA-F]{6})", html))
def lum(h):
    c = [int(h[i:i+2], 16) / 255 for i in (1, 3, 5)]
    c = [x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in c]
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
pairs = re.findall(r"aa:\s*(--c-[\w-]+)\s+on\s+(--c-[\w-]+)", html)
if not pairs: fail.append("no declared colour pairs (/* aa: --c-x on --c-y */)")
for fg, bg in pairs:
    a, b = lum(tokens[fg]), lum(tokens[bg])
    r = (max(a, b) + 0.05) / (min(a, b) + 0.05)
    print(f"{fg} on {bg}: {r:.2f}")
    if r < 4.5: fail.append(f"{fg} on {bg} is {r:.2f}:1 (< 4.5)")
for f in fail: print("FAIL:", f)
print("ok" if not fail else "")
sys.exit(1 if fail else 0)
EOF
