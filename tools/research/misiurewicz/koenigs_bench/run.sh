#!/usr/bin/env bash
# PROB-08: time the all-double Koenigs deep-pixel pipeline against fd's per-frame BLA
# renderer and a lean perturbation baseline on the same machine, then score every pixel
# against fd. Usage: run.sh FD_EXE OUT_DIR   (env: THREADS, RUNS, SIZE, DEGREE, GUARD)
set -euo pipefail
fd=$(realpath "$1"); out=$(mkdir -p "$2" && realpath "$2")
here=$(cd "$(dirname "$0")" && pwd)
threads=${THREADS:-4}; runs=${RUNS:-5}; size=${SIZE:-480x270}
degree=${DEGREE:-4}; guard=${GUARD:-1e-28}; maxit=20000
widths=(1e-35 1e-38 1e-40 1e-43 1e-46 2e-48)
re='-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502'
im='0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922'

(cd "$here" && cargo build --release --target-dir "$out/target" 2>&1 | tail -1)
bench="$out/target/release/koenigs-bench"
(cd "$here/.." && python koenigs_bench_consts.py "$degree" "$guard" 12 0.03 "$out/consts.txt")

: > "$out/path.txt"
for w in "${widths[@]}"; do echo "$re $im $w" >> "$out/path.txt"; done
echo "== fd per-frame BLA, nu only (timing)"
"$fd" control "$out/path.txt" --size "$size" --iter $maxit --columns nu --threads "$threads" \
  --bla per-frame --runs "$runs" -o "$out/fd-time" > "$out/fd-time.jsonl"
echo "== fd per-frame BLA, nu,de (reference frames for scoring)"
"$fd" control "$out/path.txt" --size "$size" --iter $maxit --columns nu,de --threads "$threads" \
  --bla per-frame --runs 1 -o "$out/fd-ref" > "$out/fd-ref.jsonl"
for mode in koenigs perturb; do
  echo "== $mode"
  "$bench" $mode "$out/consts.txt" "$out/$mode" "$size" $maxit "$threads" "$runs" "${widths[@]}" > "$out/$mode.jsonl"
  python "$here/compare.py" "$out/fd-ref" "$out/$mode" "$size" "${widths[@]}" > "$out/$mode-score.txt"
done
python "$here/report.py" "$out" | tee "$out/report.md"
