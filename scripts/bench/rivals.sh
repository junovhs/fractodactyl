#!/usr/bin/env bash
# BENC-12: fd vs installed rival renderers on bench/rivals/frames.txt (see rivals.py --help).
# Usage: bash scripts/bench/rivals.sh --fd PATH/fd [--f3 PATH/fraktaler-3.exe] [--out DIR] [--cap S]
exec python3 "$(dirname "$0")/rivals.py" "$@"
