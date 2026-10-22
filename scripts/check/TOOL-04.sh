#!/usr/bin/env bash
# Full actual repository workflow; caller supplies a positive bounded worker lease.
set -euo pipefail
cd "$(dirname "$0")/../.."
cargo +1.90.0 run --locked -p fd-dev --release --jobs "${ISHOO_CPU_ALLOCATION:?positive lease required}" -- check --jobs "$ISHOO_CPU_ALLOCATION"
