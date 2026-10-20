#!/usr/bin/env bash
# Copies the shipped places into a release package: places/README.md plus only the
# .place files git tracks, so a local untracked save can never ship (DEC-35).
# Usage: scripts/package-places.sh DEST_DIR   (creates DEST_DIR/places/)
set -euo pipefail
cd "$(dirname "$0")/.."
dest="$1/places"
mkdir -p "$dest"
cp places/README.md "$dest/"
git ls-files -z 'places/*.place' | xargs -0 -r cp -t "$dest/"
