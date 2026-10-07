#!/usr/bin/env bash
# BASE-03: run `fd bench` (3 runs: 1 cold, 2 warm, plus the oracle) on named locations
# from bench/locations.txt, collect one JSON report per line in $OUT/bench.jsonl, and
# fail unless every report is ok and has every metric populated (no null anywhere:
# peak RSS needs Linux, the oracle needs python3 with mpmath).
# Heavy: meant for CI, not a shared workstation.
# usage: scripts/bench.sh [fd-binary] [out-dir] [location-name...]
set -euo pipefail
FD=${1:-target/release/fd}
OUT=${2:-out/bench}
shift $(( $# < 2 ? $# : 2 ))
NAMES=("${@:-seahorse}")
THREADS=${THREADS:-4}
REQUIRED=(cold_seconds warm_seconds reference_seconds peak_rss_bytes sample_bytes reference_bytes
  per_pixel reference_length fds_bytes atlas_bytes_read pixel_fraction iterations_per_pixel
  class_mismatch nu_px_err de_rel_err normal_err oracle_seconds)
mkdir -p "$OUT"
: > "$OUT/bench.jsonl"
fail=0
for want in "${NAMES[@]}"; do
  line=$(awk -v n="$want" '$1 == n' bench/locations.txt)
  if [[ -z "$line" ]]; then echo "$want: FAIL not in bench/locations.txt"; fail=1; continue; fi
  read -r name re im width iter size rot kernel k <<< "$line"
  if ! json=$("$FD" bench --re "$re" --im "$im" --width "$width" --iter "$iter" --size "$size" \
      --rotation "$rot" --kernel "$kernel" --threads "$THREADS" --runs 3 \
      -o "$OUT/$name.fds" --oracle tools/oracle.py --k "$k"); then
    echo "$name: FAIL fd bench exited non-zero"; fail=1
  fi
  echo "$json" | tee -a "$OUT/bench.jsonl"
  [[ "$json" == *'"ok":true}' ]] || { echo "$name: FAIL report not ok"; fail=1; }
  [[ "$json" != *null* ]] || { echo "$name: FAIL report has unpopulated (null) metrics"; fail=1; }
  for key in "${REQUIRED[@]}"; do
    [[ "$json" == *"\"$key\":"* ]] || { echo "$name: FAIL missing $key"; fail=1; }
  done
done
exit $fail
