#!/usr/bin/env bash
# DEC-32: print CHANGELOG.md's section for VERSION (the lines under `## [VERSION]` up to
# the next `## [`). Fails if the section is missing or empty, so a version cannot be
# released without user-facing notes.  Usage: scripts/release-notes.sh 0.5.0
set -euo pipefail
v="${1:?usage: release-notes.sh VERSION}"
notes=$(awk -v h="## [$v]" '
  index($0, h) == 1 { on = 1; next }
  on && /^## \[/ { exit }
  on { print }
' "${CHANGELOG:-CHANGELOG.md}" | sed -e '/./,$!d')
[ -n "$(tr -d '[:space:]' <<<"$notes")" ] || {
  echo "CHANGELOG.md has no notes under '## [$v]': write them in the issue that bumps the version" >&2
  exit 1
}
printf '%s\n' "$notes"
