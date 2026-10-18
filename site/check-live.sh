#!/usr/bin/env bash
# SITE-08 check: fractodactyl.dev answers and serves the expected commit.
# EXPECT defaults to GITHUB_SHA (in CI) or this checkout's HEAD. Edge caches can lag a
# deploy, so /version.txt is retried for up to RETRY_S seconds (default 120).
set -u
BASE="${BASE:-https://fractodactyl.dev}"
EXPECT="${EXPECT:-${GITHUB_SHA:-$(git rev-parse HEAD)}}"
RETRY_S="${RETRY_S:-120}"
fail=0
for path in / /docs/; do
  if curl -fsS -o /dev/null --max-time 20 "$BASE$path"; then echo "ok: $BASE$path"
  else echo "FAIL: $BASE$path did not return 200"; fail=1; fi
done
deadline=$((SECONDS + RETRY_S))
while :; do
  got="$(curl -fsS --max-time 20 -H 'Cache-Control: no-cache' "$BASE/version.txt?t=$SECONDS" 2>/dev/null | tr -d '[:space:]')"
  [ "$got" = "$EXPECT" ] && { echo "ok: $BASE/version.txt = $got"; break; }
  if [ "$SECONDS" -ge "$deadline" ]; then
    echo "FAIL: $BASE/version.txt = '${got:-<none>}', expected $EXPECT"; fail=1; break
  fi
  sleep 10
done
if [ "$fail" -eq 0 ]; then
  node "$(dirname "$0")/scripts/check-download-live.mjs" || fail=1
fi
exit "$fail"
