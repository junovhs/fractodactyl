#!/usr/bin/env bash
# PERF-09: fd film's default (film) BLA tolerance against the exact one on a seahorse-valley
# slice: `fd filmcheck exact film` must PASS and the film render be >= 3x faster.
# Usage: FD=path/to/fd OUT=dir bash scripts/check/PERF-09.sh
set -euo pipefail
: "${FD:?FD=path to fd}" "${OUT:?OUT=output dir}"
mkdir -p "$OUT"
re=-0.743643887037158704752191506114774
im=0.131825904205311970493132056385139
render() { # NAME [extra flags]: prints wall seconds
    local name=$1; shift
    rm -rf "$OUT/$name"
    local t0; t0=$(date +%s.%N)
    "$FD" film "$re" "$im" --from 4e-24 --to 1e-24 --seconds 2 --fps 24 --size 480x270 \
        --iter 100000 --raw-dir "$OUT/$name" --mp4 "$OUT/$name.mp4" "$@" >/dev/null 2>"$OUT/$name.log"
    python3 -c "import sys; print(f'{float(sys.argv[2]) - float(sys.argv[1]):.2f}')" "$t0" "$(date +%s.%N)"
}
exact=$(render exact --bla-eps 8.881784197001252e-16)
film=$(render film)
echo "exact tolerance (2^-50): $exact s; film tolerance (default 2^-26): $film s"
check=$("$FD" filmcheck "$OUT/exact" "$OUT/film" --heatmap "$OUT/worst.ppm" 2>&1 | tail -1)
echo "$check"
python3 - "$exact" "$film" "$check" <<'EOF'
import sys
exact, film, check = float(sys.argv[1]), float(sys.argv[2]), sys.argv[3]
speedup = exact / film
ok = check.startswith('PASS') and speedup >= 3.0
print(f'speedup {speedup:.1f}x', 'PERF-09 PASS' if ok else 'PERF-09 FAIL')
sys.exit(0 if ok else 1)
EOF
