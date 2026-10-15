#!/bin/bash
# Zone-finish probe (GPU-09 design input). usage: run.sh SCRATCH   (CPU only, ~6 min on 4 cores)
# Captures the v0-zone 1080p finish checkpoints at the six GPU-12 depths, then scores
# float-float and the two-level float32 finish against the binary64 finish.
set -e
S=$1; D=$(cd "$(dirname "$0")" && pwd)
mkdir -p "$S/zf"
cargo build --release --manifest-path "$D/capture/Cargo.toml" --target-dir "$S/cargo-zf"
gcc -O2 -fopenmp -ffp-contract=off -o "$S/zfin" "$D/zfin.c" -lm
for w in 1e-37 1e-40 1e-43 1e-46 2e-48 2e-49; do
  [ -f "$S/zf/$w.zfin" ] || "$S/cargo-zf/release/zone-finish-capture" $w 1920 1080 "$S/zf/$w.zfin"
done
for w in 1e-37 1e-40 1e-43 1e-46 2e-48 2e-49; do for T in 8 16; do "$S/zfin" "$S/zf/$w.zfin" $T; done; done
