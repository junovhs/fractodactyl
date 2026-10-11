#!/usr/bin/env bash
# Kept for compatibility: round 2 has one authoritative fail-fast harness.
set -euo pipefail
exec bash "$(dirname "$0")/gpu06_round2.sh" "$@"
