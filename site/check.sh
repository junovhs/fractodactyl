#!/usr/bin/env bash
# SITE-01 check: fractodactyl.dev serves the explainer and the dopedocs documentation.
set -u
BASE="${BASE:-https://fractodactyl.dev}"
fail=0
page() { # PATH TEXT
  body="$(curl -fsSL --max-time 20 "$BASE$1" 2>/dev/null)" || { echo "FAIL: $BASE$1 did not return 200"; fail=1; return; }
  grep -qF -- "$2" <<<"$body" || { echo "FAIL: $BASE$1 lacks \"$2\""; fail=1; return; }
  echo "ok: $BASE$1"
}
page / "Fractodactyl"
page /docs/what-it-is/ "What is Fractodactyl?"
page /docs/ "Fractodactyl user manual"
exit "$fail"
