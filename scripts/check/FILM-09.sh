#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
exec python3 scripts/check/FILM-09.py --fd "${FD:?set FD to the fd binary}" --out "${OUT:?set OUT outside the checkout}" --before "${FD_BEFORE:?set FD_BEFORE to the unchanged binary}"
