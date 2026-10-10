#!/usr/bin/env bash
# Build a nucleus zone with the PROB-09 degree-4 map and depth-6 tail patches.
# Usage: make_zone.sh OUT.zone  (v0), or make_zone.sh RE IM PERIOD -o OUT.zone
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
if [ "$#" -eq 1 ]; then
    out=$1
    centre=()
    guard=1e-28
elif [ "$#" -eq 5 ] && [ "$4" = -o ]; then
    out=$5
    centre=("$1" "$2" "$3")
    guard=auto
else
    echo "usage: make_zone.sh [RE IM PERIOD -o] OUT.zone" >&2
    exit 2
fi
mkdir -p "$(dirname "$out")"
out=$(realpath -m "$out")
cd "$here"
python3 koenigs_bench_consts.py 4 "$guard" 12 0.03 "$out" 18 "${centre[@]}"
python3 tail_patches.py 16 1e-11 6 6 rel "$out" "${centre[@]}"
# Preserve subnormal and deeper scales: do not round the original decimal
# through f64 before converting to mantissa * 2^exponent (DEC-21).
python3 - "$out" <<'PY'
import pathlib
import sys
import mpmath as mp

path = pathlib.Path(sys.argv[1])
lines = []
for line in path.read_text().splitlines():
    fields = line.split()
    if fields and fields[0] in ("scale", "guard", "max_dc"):
        value = mp.mpf(fields[1])
        if not mp.isfinite(value) or value <= 0:
            raise ValueError(f"invalid {fields[0]}: {fields[1]}")
        mant, exp = mp.frexp(value)
        line = f"{fields[0]} {float(mant * 2)!r} {int(exp) - 1}"
    lines.append(line)
path.write_text("\n".join(lines) + "\n")
PY
