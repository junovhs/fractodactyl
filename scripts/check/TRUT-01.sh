#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
exec python3 scripts/check/TRUT-01.py --binary "${TRUTH_BIN:?set TRUTH_BIN}" --out "${OUT:?set OUT outside checkout}"
