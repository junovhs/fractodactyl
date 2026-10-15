#!/bin/sh
# Render every probe view (480x270 crop at the centre of a 1920x1080 frame) and score it.
# usage: run_views.sh SCRATCH [views.txt]
set -e
S=$1; V=${2:-$(dirname "$0")/views.txt}
B=$(dirname "$0")/target/release
mkdir -p "$S"
grep -v '^#' "$V" | while read -r name re im width it; do
  [ -f "$S/$name.ref" ] || python3 "$(dirname "$0")/refgen.py" "$re" "$im" "$width" "$it" "$S/$name.ref"
  [ -f "$S/$name.dmp" ] || "$B/render" "$S/$name.ref" "$width" 480 270 720 405 "$it" "$S/$name.dmp" ss4
done
