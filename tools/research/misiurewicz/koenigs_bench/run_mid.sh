#!/usr/bin/env bash
# PROB-14: same-machine, every-pixel mid-band comparison (nu, de, normal).
# Usage: THREADS=24 RUNS=3 SIZE=1920x1080 bash .../run_mid.sh target/release/fd out/mid
set -euo pipefail
fd=$(realpath "${1:?fd executable}"); mkdir -p "${2:?output directory}"
out=$(realpath "$2")
here=$(cd "$(dirname "$0")" && pwd)
threads=${THREADS:-4}; runs=${RUNS:-3}; size=${SIZE:-480x270}; maxit=${MAXIT:-20000}
widths=(1e-6 1e-9 1e-12 1e-15 1e-18 1e-24)
python "$here/../mid_consts.py" "$out/mid-consts.txt"
read -r _ re im < <(grep '^c_exact ' "$out/mid-consts.txt")
: > "$out/path.txt"
for w in "${widths[@]}"; do echo "$re $im $w" >> "$out/path.txt"; done
(cd "$here" && cargo build --release --target-dir "$out/target")
bench="$out/target/release/koenigs-bench"
echo "== default fd per-frame BLA (timing)"
"$fd" control "$out/path.txt" --size "$size" --iter "$maxit" --columns nu,de,normal \
  --threads "$threads" --bla per-frame --runs "$runs" -o "$out/fd-time" | tee "$out/fd.jsonl"
# The f64 reference tier at widths 1e-6/1e-9 can differ from the exact
# centre orbit. Score against an independent fixed-point reference instead.
echo "== fixed-point fd per-frame BLA (scoring)"
"$fd" control "$out/path.txt" --size "$size" --iter "$maxit" --columns nu,de,normal \
  --threads "$threads" --kernel fx --bla per-frame --runs 1 -o "$out/fd-ref" | tee "$out/fd-ref.jsonl"
echo "== koenigs-bench --mid"
"$bench" --mid "$out/mid-consts.txt" "$out/mid" "$size" "$maxit" "$threads" "$runs" "${widths[@]}" | tee "$out/mid.jsonl"
echo "== every-pixel comparison (fails the command if any gate fails)"
"$fd" compare "$out/fd-ref" "$out/mid" | tee "$out/score.jsonl"
