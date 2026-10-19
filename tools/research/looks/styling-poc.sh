#!/usr/bin/env bash
set -euo pipefail
cargo build --locked -p fd-cli --jobs "${CARGO_BUILD_JOBS:-1}"
python3 tools/research/looks/styling_poc.py
