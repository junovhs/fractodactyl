#!/usr/bin/env bash
# BASE-02 proof: render every location in bench/locations.txt twice (1 thread and
# THREADS threads), require bit-identical files, then check each against the oracle.
# Heavy: meant for CI, not a shared workstation.
# usage: scripts/locations.sh [fd-binary] [out-dir]
set -euo pipefail
FD=${1:-target/release/fd}
OUT=${2:-out/locations}
THREADS=${THREADS:-4}
mkdir -p "$OUT"
: > "$OUT/oracle.jsonl"
fail=0
while read -r name re im width iter size rot kernel k; do
  [[ -z "${name:-}" || "$name" == \#* ]] && continue
  args=(render --re "$re" --im "$im" --width "$width" --iter "$iter" --size "$size" --rotation "$rot" --kernel "$kernel")
  "$FD" "${args[@]}" --threads 1 -o "$OUT/$name.a.fds"
  "$FD" "${args[@]}" --threads "$THREADS" -o "$OUT/$name.b.fds"
  if cmp -s "$OUT/$name.a.fds" "$OUT/$name.b.fds"; then
    echo "$name: bit-identical across 1 and $THREADS threads"
  else
    echo "$name: FAIL output differs between thread counts"
    fail=1
  fi
  start=$(date +%s)
  if ! python3 tools/oracle.py "$OUT/$name.a.fds" --k "$k" | tee -a "$OUT/oracle.jsonl"; then
    echo "$name: FAIL oracle"
    fail=1
  fi
  echo "$name: oracle took $(( $(date +%s) - start )) s"
done < bench/locations.txt
exit $fail
