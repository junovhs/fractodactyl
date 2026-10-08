#!/usr/bin/env bash
# KERN-01: write the v0 zone file that `fd render/control --zone` reads, with the
# settings PROB-09 validated at 1080p (degree-4 biseries, guard 1e-28, 12 phi terms,
# R0 0.03, 18-term psi series, depth-6 tail patch atlas). About 15-20 s.
# Usage: make_zone.sh OUT.zone      (needs python3 with mpmath and numpy)
set -euo pipefail
out=$(realpath -m "$1")
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"
python3 koenigs_bench_consts.py 4 1e-28 12 0.03 "$out" 18
python3 tail_patches.py 16 1e-11 6 6 rel "$out"
