#!/usr/bin/env bash
# GPU-02 desktop gate: 10 s of the v0 path, raw RGB24 film equivalence.
set -euo pipefail
cd "$(dirname "$0")/../.."

DURATION=${DURATION:-10}
SIZE=${SIZE:-1920x1080}
FPS=${FPS:-60}
TO=${TO:-2e-49}
GPU_KERNEL=${GPU_KERNEL:-mixed}
case "$GPU_KERNEL" in
    f32|mixed) ;;
    *) echo "GPU_KERNEL must be f32 or mixed" >&2; exit 2 ;;
esac

out=${OUT:-out/gpu02}
mkdir -p "$out"
cargo build --release -p fd-cli
fd=${CARGO_TARGET_DIR:-target}/release/fd
re=-0.7432918908524302029316241585089040394625440130877230883413356446722846985935655895273743988748934502
im=0.1312405523087976047708458738159648480193742492666251343726688732491323053181613282916110110463622922
args=("$re" "$im" --from 1e-29 --to "$TO" --seconds "$DURATION" --fps "$FPS" \
      --size "$SIZE" --keyframes 2 --iter 100000 --zone bench/zones/v0-core.zone \
      --preset master-ice)
rm -rf "$out/cpu-raw" "$out/gpu-raw"
"$fd" film "${args[@]}" --mp4 "$out/cpu.mp4" --raw-dir "$out/cpu-raw" \
    > "$out/cpu.json" 2> "$out/cpu.log"
"$fd" film "${args[@]}" --gpu-kernel "$GPU_KERNEL" --mp4 "$out/gpu.mp4" \
    --raw-dir "$out/gpu-raw" > "$out/gpu.json" 2> "$out/gpu.log"

# A CPU fallback is correct but does not prove the shortcut.
if ! grep -q "Vulkan $GPU_KERNEL zone on" "$out/gpu.log"; then
    echo "GPU-02: no Vulkan $GPU_KERNEL keyframes rendered (see $out/gpu.log)" >&2
    exit 1
fi
python3 - "$out" <<'PY'
import json
import pathlib
import sys

out = pathlib.Path(sys.argv[1])
for name in ("cpu", "gpu"):
    report = json.loads((out / f"{name}.json").read_text())
    print(f"{name} keyframe render seconds: {report['render']['total_seconds']:.3f}", flush=True)
PY
"$fd" filmcheck "$out/cpu-raw" "$out/gpu-raw" --heatmap "$out/worst.ppm"
