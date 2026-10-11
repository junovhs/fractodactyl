#!/usr/bin/env bash
# GPU-06 final RX gate: small and 1080p RGB, encoder baseline, 4K.
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p out/gpu06/r2
dir=out/gpu06/r2
fd=target/release/fd
re=-0.8296874999999996981089858669738646984522311568651036619209
im=0.1859374999999989417838556112859064221737945709381783514212
common=("$re" "$im" --from 1e-20 --to 2.5e-21 --fps 30 --keyframes 2 --preset master-ice)
export VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation
fail() { echo "FAIL GPU-06 $1. Logs and outputs: $dir" >&2; exit 1; }
trap 'echo "GPU-06 error at script line $LINENO; inspect $dir" >&2' ERR
cargo build --release -p fd-cli || fail "release build"
# Raw parity uses the same samples, phase and tent path as the real 1200-frame film.
# Test both the start and later look time without encoding artifacts.
echo "GPU-06 stage 1/3: raw RGB24 (before x264), 320x180, frames 0..3 and 600..603"
for range in 0..3 600..603; do
  start=${range%%..*}
  for gpu in off on; do
    mkdir -p "$dir/raw-$start-$gpu"
    rm -f "$dir/raw-$start-$gpu"/frame-*.rgb
    "$fd" film "${common[@]}" --seconds 40 --size 320x180 --frames "$range" \
      --gpu "$gpu" --raw-dir "$dir/raw-$start-$gpu" \
      --mp4 "$dir/raw-$start-$gpu.mp4" \
      >"$dir/raw-$start-$gpu.json" 2>"$dir/raw-$start-$gpu.stderr" \
      || fail "raw $range GPU=$gpu (stderr: $dir/raw-$start-$gpu.stderr)"
  done
  python3 - "$dir/raw-$start-on.json" <<'PY' || fail "raw $range GPU fell back"
import json,sys
r=json.load(open(sys.argv[1]))
assert r.get('gpu_device'), 'no Vulkan GPU device in JSON'
PY
  python3 tools/research/gpu06/compare_raw.py \
    "$dir/raw-$start-off" "$dir/raw-$start-on" --size 320x180 \
    | tee "$dir/raw-$start-diff.txt" || fail "raw RGB $range exceeds ±1 channel"
  if grep -Eq 'Validation Error|VUID-' "$dir/raw-$start-on.stderr"; then fail "raw Vulkan validation $range"; fi
done
echo "GPU-06 stage 2/3: full-resolution raw RGB, 1200-frame encoded SSIM and shade rate"
# DEC-18: compare actual 1920x1080 RGB24 before x264, at both times.
for range in 0..3 600..603; do
  start=${range%%..*}
  for gpu in off on; do
    raw="$dir/full-$start-$gpu"
    mkdir -p "$raw"
    rm -f "$raw"/frame-*.rgb
    "$fd" film "${common[@]}" --seconds 40 --size 1920x1080 --frames "$range" \
      --gpu "$gpu" --raw-dir "$raw" --mp4 "$dir/full-$start-$gpu.mp4" \
      >"$dir/full-$start-$gpu.json" 2>"$dir/full-$start-$gpu.stderr" \
      || fail "1080p raw range $range GPU=$gpu (see full-$start-$gpu.stderr)"
  done
  python3 - "$dir/full-$start-on.json" <<'PY' || fail "1080p raw range $range fell back to CPU"
import json,sys
with open(sys.argv[1]) as f:
    report=json.load(f)
assert report.get('gpu_device'), 'missing GPU device in raw 1080p report'
PY
  python3 tools/research/gpu06/compare_raw.py \
    "$dir/full-$start-off" "$dir/full-$start-on" --size 1920x1080 \
    | tee "$dir/full-$start-diff.txt" || fail "1080p pre-x264 RGB range $range exceeds +/-1 level"
  if grep -Eq 'Validation Error|VUID-' "$dir/full-$start-on.stderr"; then
    fail "1080p raw Vulkan validation range $range"
  fi
done
for gpu in on off; do
  "$fd" film "${common[@]}" --seconds 40 --size 1920x1080 \
    --gpu "$gpu" --mp4 "$dir/$gpu-1200.mp4" \
    >"$dir/$gpu-1200.json" 2>"$dir/$gpu-1200.stderr" \
    || fail "1200-frame 1080p GPU=$gpu (see $gpu-1200.stderr)"
done
if grep -Eq 'Validation Error|VUID-' "$dir/on-1200.stderr"; then
  fail "1200-frame 1080p Vulkan validation"
fi
# Both encodes are scored against the SAME six unencoded CPU RGB frames.
# Compare identical timestamps: 0..2 and 600..602, never two x264 files
# against one raw-to-encode baseline (which double-counts codec drift).
cpu_source=(
  "$dir/full-0-off/frame-000000.rgb"
  "$dir/full-0-off/frame-000001.rgb"
  "$dir/full-0-off/frame-000002.rgb"
  "$dir/full-600-off/frame-000600.rgb"
  "$dir/full-600-off/frame-000601.rgb"
  "$dir/full-600-off/frame-000602.rgb"
)
for gpu in off on; do
  cat "${cpu_source[@]}" |
    ffmpeg -hide_banner -nostdin -f rawvideo -pix_fmt rgb24 \
      -video_size 1920x1080 -framerate 30 -i pipe:0 -i "$dir/$gpu-1200.mp4" \
      -filter_complex "[0:v]format=yuv420p,settb=1/30,setpts=N[raw];[1:v]split=2[early][late];[early]trim=start_frame=0:end_frame=3,setpts=PTS-STARTPTS[first];[late]trim=start_frame=600:end_frame=603,setpts=PTS-STARTPTS[last];[first][last]concat=n=2:v=1:a=0,settb=1/30,setpts=N[encoded];[raw][encoded]ssim=stats_file=$dir/ssim-$gpu.txt[comparison]" \
      -map '[comparison]' -vsync 0 -f null - 2>"$dir/ssim-$gpu.stderr" \
    || fail "ffmpeg $gpu MP4 vs CPU raw SSIM"
done
python3 - "$dir" <<'PY' || fail "1080p encoder fidelity below CPU baseline minus 0.001 or shade >0.01 s/frame"
import json,re,sys
from pathlib import Path
d=Path(sys.argv[1])
r=json.loads((d/'on-1200.json').read_text())
assert r.get('gpu_device'), 'GPU fell back: missing GPU device'
def score(gpu):
    records=(d/f'ssim-{gpu}.txt').read_text().splitlines()
    assert len(records)==6 and all(row.startswith(f'n:{i} ') for i,row in enumerate(records,1)), \
        f'{gpu}: expected exactly six time-aligned SSIM frames, got {len(records)}'
    values=re.findall(r'All:([0-9.]+)', (d/f'ssim-{gpu}.stderr').read_text())
    assert len(values)==1, f'{gpu}: expected one aggregate SSIM All score'
    return float(values[0])
baseline=score('off')
actual=score('on')
floor=baseline-0.001
print('1080p:',r['gpu_device'],'shade_s/frame',r['shade_seconds_per_frame'],
      'GPU encode-vs-CPU source SSIM All',actual,
      'CPU encode-vs-CPU source SSIM All',baseline,
      'minimum allowed SSIM',floor,flush=True)
assert 0.98 <= baseline <= 1.0, 'unexpected encoder baseline: check input frame correspondence'
assert actual >= floor, f'GPU source SSIM {actual:.6f} < CPU source SSIM {baseline:.6f} - 0.001'
assert r['shade_seconds_per_frame'] <= .01, 'shade slower than 0.010 s/frame'
PY
echo "GPU-06 stage 3/3: 4K first 120 frames shade rate"
"$fd" film "${common[@]}" --rate 0.05 --size 3840x2160 \
  --frames 0..120 --gpu on --mp4 "$dir/on-4k.mp4" \
  >"$dir/on-4k.json" 2>"$dir/on-4k.stderr" || fail "4K run"
if grep -Eq 'Validation Error|VUID-' "$dir/on-4k.stderr"; then fail "4K Vulkan validation"; fi
python3 - "$dir" <<'PY' || fail "4K shade >= 0.04 s/frame or GPU fallback"
import json,sys
from pathlib import Path
d=Path(sys.argv[1]); r=json.loads((d/'on-4k.json').read_text())
assert r.get('gpu_device'), 'missing Vulkan GPU device'
print('4K:',r['gpu_device'],'shade_s/frame',r['shade_seconds_per_frame'],
      'keyframe_stages',r.get('keyframe_stages'),flush=True)
assert r['shade_seconds_per_frame']<.04,'4K shade slower than 0.040 s/frame'
PY
echo "PASS GPU-06: small/full-size RGB24 parity <=1, Vulkan validation, encoder-baseline SSIM, 1080p/4K rates"
