#!/usr/bin/env bash
# FIX-09 acceptance experiment: 24 whole frames across 1e-14 .. 1e-29.
# Both arms render independent frames with their own reference; BLA also builds
# its own table inside the timed run. No atlas, zone, or cross-frame sharing.
# Run: bash scripts/check/FIX-09.sh
# Optional: FD_FIX09_THREADS=4 FD_FIX09_RUNS=2 FD_FIX09_ITER=100000
set -euo pipefail

cd "$(dirname "$0")/../.."
out="${FD_FIX09_OUT:-out/FIX-09}"
threads="${FD_FIX09_THREADS:-4}"
runs="${FD_FIX09_RUNS:-1}"
iter="${FD_FIX09_ITER:-100000}"
mkdir -p "$out"

python3 - bench/path-atlas-v0.txt "$out/24-frames.txt" <<'PY'
import math
import sys
from pathlib import Path

source = Path(sys.argv[1]).read_text(encoding="utf-8").splitlines()
frames = [line for line in source if line.strip() and not line.startswith("#")]
depths = [-math.log10(float(line.split()[2])) for line in frames]
available = set(range(len(frames)))
selected = []
# Five non-overlapping 3-decade bands, 5+5+5+5+4 = 24 frames.
for band, count in enumerate((5, 5, 5, 5, 4)):
    for j in range(count):
        target = 14 + 3 * band + 3 * (j + 0.5) / count
        i = min(available, key=lambda k: abs(depths[k] - target))
        assert 14 + 3 * band <= depths[i] < 17 + 3 * band
        selected.append(i)
        available.remove(i)
selected.sort()
Path(sys.argv[2]).write_text(
    "\n".join(frames[i] for i in selected) + "\n", encoding="utf-8"
)
print("FIX-09 selected 24 v0 path frames at depths:",
      " ".join(f"{depths[i]:.2f}" for i in selected))
PY

fd="${FD_FIX09_FD:-target/release/fd}"
if [ ! -x "$fd" ]; then
    cargo build --locked --release -p fd-cli
fi
common=(--size 480x270 --ss 1 --iter "$iter" --columns nu,de --threads "$threads" --runs "$runs")

echo "FIX-09: plain perturbation"
"$fd" control "$out/24-frames.txt" "${common[@]}" -o "$out/plain" | tee "$out/plain.jsonl"
echo "FIX-09: independent per-frame BLA / plain fallback"
"$fd" control "$out/24-frames.txt" "${common[@]}" --bla per-frame -o "$out/la" | tee "$out/la.jsonl"
echo "FIX-09: whole-frame sample comparison (classes, nu, de)"
"$fd" compare "$out/plain" "$out/la" | tee "$out/compare.jsonl"

python3 - "$out" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
def records(name):
    return [json.loads(line) for line in (root / name).read_text().splitlines()]
a, b, cmp = (records("plain.jsonl"), records("la.jsonl"), records("compare.jsonl"))
assert len(a) == len(b) == len(cmp) == 25, "expected 24 frames plus a totals row"
assert all(r.get("record") == "frame" for r in a[:24] + b[:24] + cmp[:24])
assert cmp[-1].get("ok") is True, "fd compare found a correctness failure"
ok = True
offset = 0
for band, count in enumerate((5, 5, 5, 5, 4)):
    left, right = a[offset:offset+count], b[offset:offset+count]
    plain = sum(x["timing"]["cold_seconds"] for x in left)
    la = sum(x["timing"]["cold_seconds"] for x in right)
    ratio = plain / la if la > 0 else 0
    passes = ratio >= 3.0
    ok &= passes
    print(f"1e-{14+3*band} .. 1e-{17+3*band}: "
          f"plain {plain:.3f}s, LA {la:.3f}s, "
          f"speedup {ratio:.2f}x, {'PASS' if passes else 'FAIL'}")
    offset += count
print("FIX-09 full-frame correctness: PASS (fd compare zero failures)")
print(f"FIX-09 performance: {'PASS' if ok else 'FAIL'} (>=3.0x in every band)")
sys.exit(0 if ok else 1)
PY
