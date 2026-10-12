#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
if rg 'fn scaled|_fx\(' crates/fd-kernel; then
    echo 'tier-specific kernel copies remain' >&2
    exit 1
fi
bundle=${1:-crates/fd-kernel/fixtures/KERN-05.bundle}
timing=${2:?supply a fresh baseline timing file from kernel_contract capture}
export CARGO_BUILD_JOBS=1
export RUST_TEST_THREADS=1
cargo run --locked -p fd-kernel --example kernel_contract --jobs 1 -- compare "$bundle" "$timing"
