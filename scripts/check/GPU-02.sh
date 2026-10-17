#!/usr/bin/env bash
# GPU-02 desktop gate: 10 s of the v0 path, raw RGB24 film equivalence of the f32 or
# mixed zone kernel against the binary64 one, which meets the strict bar on the frozen
# zone fixtures (CLEA-06). The CPU zone it was first checked against is deleted (CLEA-04).
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
rm -rf "$out/ref-raw" "$out/gpu-raw"
"$fd" film "${args[@]}" --gpu-kernel f64 --mp4 "$out/ref.mp4" --raw-dir "$out/ref-raw" \
    > "$out/ref.json" 2> "$out/ref.log"
"$fd" film "${args[@]}" --gpu-kernel "$GPU_KERNEL" --mp4 "$out/gpu.mp4" \
    --raw-dir "$out/gpu-raw" > "$out/gpu.json" 2> "$out/gpu.log"

# A fallback is correct but does not prove the kernel under test.
if ! grep -q "wgpu FP64 zone on" "$out/ref.log"; then
    echo "GPU-02: no wgpu FP64 reference keyframes rendered (see $out/ref.log)" >&2
    exit 1
fi
if ! grep -q "wgpu $GPU_KERNEL zone on" "$out/gpu.log"; then
    echo "GPU-02: no wgpu $GPU_KERNEL keyframes rendered (see $out/gpu.log)" >&2
    exit 1
fi
python3 - "$out" <<'PY'
import json
import pathlib
import sys

out = pathlib.Path(sys.argv[1])
for name in ("ref", "gpu"):
    report = json.loads((out / f"{name}.json").read_text())
    print(f"{name} keyframe render seconds: {report['render']['total_seconds']:.3f}", flush=True)
PY
"$fd" filmcheck "$out/ref-raw" "$out/gpu-raw" --heatmap "$out/worst.ppm"
