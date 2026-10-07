#!/usr/bin/env bash
# BENC-04: time fraktaler-3 (mathr's perturbation + BLA renderer) on the PROB-08 frames, on
# the same machine as fd per-frame BLA and our pipeline, and compare fraktaler-3 point for
# point with our pipeline at its own jittered sample points.
# Usage: f3_bench.sh F3_EXE FD_EXE OUT_DIR   (env: SIZE, RUNS; threads = all cores, as f3 uses)
set -euo pipefail
f3=$(realpath "$1"); fd=$(realpath "$2"); out=$(mkdir -p "$3" && realpath "$3")
here=$(cd "$(dirname "$0")" && pwd)
size=${SIZE:-480x270}; runs=${RUNS:-3}; nx=${size%x*}; ny=${size#*x}
threads=$(nproc)
widths=(1e-35 1e-38 1e-40 1e-43 1e-46 2e-48)
re='-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502'
im='0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922'
now() { echo "$EPOCHREALTIME"; }   # bash 5 builtin: no process-start bias

echo "== fd per-frame BLA and our pipeline (run.sh, series psi + patch atlas), $threads threads"
THREADS=$threads RUNS=$runs SIZE=$size PSI=18 PATCH=6 bash "$here/run.sh" "$fd" "$out/ours"

echo "== fraktaler-3 hardware tuning"
(cd "$out" && "$f3" --version) || true
if (cd "$out" && "$f3" -W -w "$out/wisdom.toml" && timeout 900 "$f3" -B -w "$out/wisdom.toml"); then
  echo "wisdom benchmarked"
else
  echo "wisdom benchmark failed; using whatever wisdom exists"
fi
wisdom=()
if [ -f "$out/wisdom.toml" ]; then wisdom=(-w "$out/wisdom.toml"); head -40 "$out/wisdom.toml"; fi

: > "$out/f3-times.txt"
for w in "${widths[@]}"; do
  zoom=$(python -c "print(repr(4/($w*$ny/$nx)))")
  {
    echo "location.real = \"$re\""
    echo "location.imag = \"$im\""
    echo "location.zoom = \"$zoom\""
    echo "bailout.iterations = 20000"
    echo "bailout.maximum_reference_iterations = 20000"
    echo "bailout.maximum_perturb_iterations = 20000"
    echo "bailout.maximum_bla_steps = 20000"
    echo "bailout.escape_radius = 1e10"
    echo "image.width = $nx"
    echo "image.height = $ny"
    echo "image.subframes = 1"
    echo "render.filename = \"f3-w$w\""
    echo "render.save_exr = true"
    echo "render.exr_channels = [\"N0\", \"N1\", \"NF\", \"DEX\", \"DEY\"]"
  } > "$out/w$w.f3.toml"
  times=()
  for r in $(seq "$runs"); do
    t0=$(now)
    (cd "$out" && "$f3" --batch -P "${wisdom[@]}" "w$w.f3.toml" > "f3-w$w.log" 2>&1)
    t1=$(now)
    times+=("$(python -c "print($t1 - $t0)")")
  done
  echo "$w $(python -c 'import sys; print(min(map(float, sys.argv[1:])))' "${times[@]}")" | tee -a "$out/f3-times.txt"
done
# fixed per-process cost (startup, wisdom, EXR write): a 16x9 frame at zoom 1, 100 iterations
printf 'image.width = 16
image.height = 9
image.subframes = 1
bailout.iterations = 100
render.filename = "f3-tiny"
render.save_exr = true
render.exr_channels = ["N0"]
' > "$out/tiny.f3.toml"
times=()
for r in $(seq "$runs"); do
  t0=$(now); (cd "$out" && "$f3" --batch -P "${wisdom[@]}" tiny.f3.toml > f3-tiny.log 2>&1); t1=$(now)
  times+=("$(python -c "print($t1 - $t0)")")
done
echo "tiny $(python -c 'import sys; print(min(map(float, sys.argv[1:])))' "${times[@]}")" | tee -a "$out/f3-times.txt"
ls -la "$out" | grep -i exr || { echo "no EXR written"; tail -20 "$out"/f3-w*.log; }

echo "== our pipeline at fraktaler-3's sample points (frame / y-sign variants)"
for fr in 0 -1; do for ys in 1 -1; do
  F3_FRAME=$fr F3_YSIGN=$ys "$out/ours/target/release/koenigs-bench" koenigs "$out/ours/consts.txt" \
    "$out/j$fr$ys" "$size" 20000 "$threads" 1 "${widths[@]}" > /dev/null
done; done
python "$here/compare_f3.py" "$out" "$size" "${widths[@]}" | tee "$out/f3-score.txt"
python "$here/report_f3.py" "$out" "$size" "${widths[@]}" | tee "$out/report.md"
