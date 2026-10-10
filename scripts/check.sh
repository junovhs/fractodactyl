#!/usr/bin/env bash
# Required landing checks: rustfmt, cargo check, clippy (warnings are errors), and tests.
# Parallelism is bounded (JOBS, default 2) so it is safe on a shared workstation.
set -euo pipefail
JOBS=${JOBS:-2}
export RUST_TEST_THREADS=$JOBS
cargo fmt --all --check
cargo check --workspace --all-targets -j "$JOBS"
cargo clippy --workspace --all-targets -j "$JOBS" -- -D warnings
cargo test --workspace -j "$JOBS"
