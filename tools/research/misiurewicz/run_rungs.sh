#!/usr/bin/env bash
# PROB-17: paired whole-frame zone/control measurements at three v0 ladder rungs.
# Usage: THREADS=24 RUNS=3 SIZE=480x270 bash run_rungs.sh target/release/fd out/prob17
# Optional: RUNGS="0 409 7676" FRAMES="5000 500 50 5"
set -euo pipefail
fd=$(realpath "$1")
out=$(mkdir -p "$2" && realpath "$2")
here=$(cd "$(dirname "$0")" && pwd)
threads=${THREADS:-4}; runs=${RUNS:-3}; grid=${SIZE:-480x270}
rungs=${RUNGS:-"0 409 7676"}
frames=${FRAMES:-"5000 500 50 5"}
rc=0

for k in $rungs; do
    case "$k" in
      0)
        period=764; size=4.1205e-50
        re='-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502'
        im='0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922'
        ;;
      409|7676)
        read -r period re im < <(awk -v k="$k" '$1 == k {print $2, $3, $4}' "$here/ladder_rungs.txt")
        if [ "$k" = 409 ]; then size=9.2e-101; else size=1.01e-1000; fi
        ;;
      *) echo "unknown rung $k" >&2; exit 2 ;;
    esac
    run="$out/$k"
    mkdir -p "$run"
    : > "$run/path.txt"
    for mult in $frames; do
        width=$(python3 - "$size" "$mult" <<'PY'
from decimal import Decimal
import sys
print(f"{Decimal(sys.argv[1]) * Decimal(sys.argv[2]):.12E}")
PY
)
        printf '%s %s %s\n' "$re" "$im" "$width" >> "$run/path.txt"
    done
    echo "== rung $k, period $period, widths $frames minibrot sizes"
    begin=$(date +%s%N)
    if [ "$k" = 0 ]; then
        ok=0; bash "$here/make_zone.sh" "$run/rung.zone" > "$run/build.log" 2>&1 || ok=$?
    else
        ok=0; bash "$here/make_zone.sh" "$re" "$im" "$period" -o "$run/rung.zone" > "$run/build.log" 2>&1 || ok=$?
    fi
    python3 - "$begin" "$run" "$k" "$ok" <<'PY'
import pathlib, sys, time
start, run, rung, status = sys.argv[1:]
seconds = (time.time_ns() - int(start)) / 1e9
pathlib.Path(run, "build-seconds.txt").write_text(f"{seconds:.3f}\n")
print(f"rung {rung}: build {seconds:.3f} s, exit {status}")
PY
    if [ "$ok" -ne 0 ]; then
        cat "$run/build.log" >&2; rc=1; continue
    fi
    if [ "${BUILD_ONLY:-0}" = 1 ]; then continue; fi
    maxit=$((40 * period))
    common=(--size "$grid" --iter "$maxit" --columns nu,de,normal --threads "$threads" --runs "$runs")
    if ! "$fd" control "$run/path.txt" "${common[@]}" --zone "$run/rung.zone" -o "$run/zone" > "$run/zone.jsonl"; then
        echo "rung $k: zone render failed" >&2; rc=1; continue
    fi
    if ! "$fd" control "$run/path.txt" "${common[@]}" --bla per-frame -o "$run/bla" > "$run/bla.jsonl"; then
        echo "rung $k: control render failed" >&2; rc=1; continue
    fi
    "$fd" compare "$run/bla" "$run/zone" > "$run/score.jsonl" || rc=1
    python3 - "$run" "$k" <<'PY'
import json, pathlib, sys
path, k = pathlib.Path(sys.argv[1]), sys.argv[2]
def rows(name):
    return [json.loads(x) for x in (path / name).read_text().splitlines() if x.strip()]
zs = [x for x in rows("zone.jsonl") if x.get("record") == "frame"]
bs = [x for x in rows("bla.jsonl") if x.get("record") == "frame"]
cs = [x for x in rows("score.jsonl") if x.get("record") == "frame"]
for i, (z, b, c) in enumerate(zip(zs, bs, cs)):
    mism = c.get("kinds", {})
    fix04 = mism.get("interior_to_unresolved", 0) + mism.get("unresolved_to_interior", 0)
    real = c.get("class_mismatches", 0) - fix04
    n = c.get("samples", 0)
    ret = z.get("zone", {}).get("returns", 0) / n if n else 0
    # Time is reported by fd per frame, with repeated runs in each measurement.
    print(f"rung={k} frame={i} zone_used={z.get('zone', {}).get('used')} "
          f"returns_per_pixel={ret:.5f} "
          f"zone_seconds={min(x['seconds'] for x in z['runs'])} "
          f"bla_seconds={min(x['seconds'] for x in b['runs'])} "
          f"bla_use={b.get('bla', {}).get('use', 'none')} "
          f"nu_over_px={c.get('nu', {}).get('over_px', 'unknown')} "
          f"class_real={real} fix04={fix04} max_px={c.get('nu', {}).get('px_max')}")
PY
done
exit "$rc"
