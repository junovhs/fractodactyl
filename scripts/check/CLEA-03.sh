#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
# Assemble the retired names so this scanner does not match its own pattern.
workflow=web
branch=gpt
pattern="${workflow}.mode|${workflow}-mode|${branch}/"
status=0
if matches=$(rg -il --glob '!METHOD.md' "$pattern" -- CLAUDE.md docs/spec .github tools scripts); then
    echo "Retired workflow references remain:" >&2
    echo "$matches" >&2
    status=1
else
    result=$?
    if [[ $result != 1 ]]; then exit "$result"; fi
fi
for path in "tools/${workflow}-mode" "docs/spec/${workflow^^}-MODE.md"; do
    if [[ -e $path ]]; then echo "Retired path remains: $path" >&2; status=1; fi
done
# These current product entry points are unrelated to the retired delegation tooling.
for path in crates/fd-cli/src/explore.rs crates/fd-cli/src/explore.html crates/fd-viewer/src/main.rs docs/spec/METHOD.md; do
    if [[ ! -f $path ]]; then echo "Required retained path missing: $path" >&2; status=1; fi
done
if [[ $status == 0 ]]; then echo 'CLEA-03: retired workflow absent; current viewers and historical record retained'; fi
exit "$status"
