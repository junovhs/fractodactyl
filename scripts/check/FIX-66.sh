#!/usr/bin/env bash
# FIX-66 check: a release package ships no personal or unverified place (DEC-35).
# Plants an untracked save in places/, runs the release package step for places, and
# asserts the package and the index hold zero .place files. Exits 1 on any found.
set -u
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT" || exit 2
fail=0
tmp="$(mktemp -d)"
probe="places/fix66_untracked_probe.place"
trap 'rm -rf "$tmp" "$probe"' EXIT
printf 'probe\n' > "$probe"
bash scripts/package-places.sh "$tmp" || { echo "package-places.sh failed"; exit 1; }
shipped=$(find "$tmp/places" -name '*.place' | wc -l)
tracked=$(git ls-files 'places/*.place' | wc -l)
echo "packaged .place files: $shipped; tracked .place files: $tracked"
[ "$shipped" -eq 0 ] || { find "$tmp/places" -name '*.place'; fail=1; }
[ "$tracked" -eq 0 ] || { git ls-files 'places/*.place'; fail=1; }
[ -f "$tmp/places/README.md" ] || { echo "places/README.md missing from package"; fail=1; }
exit $fail
